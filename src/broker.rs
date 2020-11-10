use super::error::{AnyError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessExt, System, SystemExt};
use zmq;

#[derive(Debug)]
struct WorkerRef {
    pid: u32,
    os_process: Child,
}

#[derive(Debug)]
pub struct Broker {
    pub id: u32,
    stop_signal: Arc<AtomicBool>,
    pub worker_instances: usize,
    pub worker_binpath: PathBuf,
    pub worker_outpath: PathBuf,
    pub worker_timeout: Duration,
    running_workers: Arc<RwLock<HashMap<u32, WorkerRef>>>,
}

impl Broker {
    pub fn new(
        id: u32,
        stop_signal: Arc<AtomicBool>,
        worker_instances: usize,
        worker_binpath: &Path,
        worker_outpath: &Path,
        worker_timeout: Duration,
    ) -> Broker {
        let instance = Broker {
            id: id,
            stop_signal: stop_signal,
            worker_instances: worker_instances,
            worker_binpath: PathBuf::from(worker_binpath),
            worker_outpath: PathBuf::from(worker_outpath),
            worker_timeout: worker_timeout,
            running_workers: Arc::new(RwLock::new(HashMap::new())),
        };
        instance
    }

    pub fn run<F: Fn(Vec<u32>)>(&mut self, on_ready: F) -> Result<()> {
        self.start(on_ready).expect("failed to start broker");
        self.stop().expect("failed to stop broker");
        Ok(())
    }

    fn start<F: Fn(Vec<u32>)>(&mut self, on_ready: F) -> Result<()> {
        self.start_workers().expect("failed to start workers");
        self.watch_workers().expect("failed watching processess");
        self.start_proxy(|| {
            let pids = self
                .running_workers
                .read()
                .expect("failed to acquire read lock of running workers map")
                .iter()
                .map(|kv| *kv.0)
                .collect();
            // workers are ready, so notify it
            on_ready(pids);
        })
        .expect("failed to start proxy");
        Ok(())
    }

    fn start_workers(&mut self) -> Result<()> {
        for i in 0..self.worker_instances {
            let child = Self::start_worker(
                &self.worker_binpath,
                &self.worker_outpath,
                &self.worker_timeout,
            )
            .expect(format!("failed to start worker #{} {:?}", i, self.worker_binpath).as_str());
            Self::add_running_worker(self.running_workers.clone(), child);
        }
        Ok(())
    }

    fn start_worker(
        worker_binpath: &PathBuf,
        worker_outpath: &PathBuf,
        worker_timeout: &Duration,
    ) -> Result<Child> {
        let result = Command::new(&worker_binpath)
            .arg("start")
            .arg("--output")
            .arg(&worker_outpath.to_str().unwrap())
            .arg("--timeout")
            .arg(worker_timeout.as_secs().to_string())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn();
        match result {
            Ok(child) => Ok(child),
            Err(reason) => Err(AnyError::from(reason)),
        }
    }

    fn add_running_worker(running_workers: Arc<RwLock<HashMap<u32, WorkerRef>>>, child: Child) {
        running_workers
            .write()
            .expect("failed to acquire write lock to add running worker")
            .insert(
                child.id(),
                WorkerRef {
                    pid: child.id(),
                    os_process: child,
                },
            );
    }

    fn remove_running_worker(running_workers: Arc<RwLock<HashMap<u32, WorkerRef>>>, pid: u32) {
        running_workers
            .write()
            .expect("failed to acquire write lock to remove dead worker")
            .remove(&pid);
    }

    fn start_proxy<F: Fn()>(&self, on_ready: F) -> Result<()> {
        let ctx = zmq::Context::new();

        let mut frontend = ctx.socket(zmq::ROUTER).unwrap();
        frontend
            .bind("tcp://127.0.0.1:6660")
            .expect("failed binding frontend socket");

        let mut backend = ctx.socket(zmq::DEALER).unwrap();
        backend
            .bind("tcp://127.0.0.1:6661")
            .expect("failed binding backend socket");

        let mut control = ctx.socket(zmq::SUB).unwrap();
        control
            .connect("tcp://127.0.0.1:6662")
            .expect("failed connecting control socket");
        control
            .set_subscribe(b"")
            .expect("failed subscribing to control socket");

        // ready to start proxying, so notify it
        on_ready();

        zmq::proxy_steerable(&mut frontend, &mut backend, &mut control)
            .expect("failed on proxying");
        Ok(())
    }

    fn watch_workers(&self) -> Result<()> {
        let id = self.id as usize;
        let worker_exec = self
            .worker_binpath
            .file_name()
            .unwrap()
            .to_owned()
            .into_string()
            .unwrap();

        let stop_signal = self.stop_signal.clone();
        let worker_binpath = self.worker_binpath.clone();
        let worker_outpath = self.worker_outpath.clone();
        let worker_timeout = self.worker_timeout;
        let running_workers = self.running_workers.clone();

        std::thread::spawn(move || {
            while !stop_signal.load(Ordering::SeqCst) {
                // it's better to wait at start then at end, because at the end
                // the sleeping might interfere on the shutting down process
                thread::sleep(Duration::from_secs(5));

                println!("--> [watch_workers]");
                // actual workers running now under this broker
                let mut current_running_workers = Vec::new();
                let sys = System::new_all();
                for (pid, process) in sys.get_processes() {
                    if process.name().contains(worker_exec.as_str()) {
                        if let Some(parent) = process.parent() {
                            if parent == id {
                                current_running_workers.push(pid);
                                println!(
                                    "[{}:{}] {} {}kB {}%",
                                    parent,
                                    pid,
                                    process.name(),
                                    process.memory(),
                                    process.cpu_usage()
                                );
                            }
                        }
                    }
                }

                // house keeping
                if !stop_signal.load(Ordering::SeqCst) {
                    let pids: Vec<u32> = running_workers
                        .read()
                        .expect("failed to acquire lock of running workers")
                        .iter()
                        .map(|kv| *kv.0)
                        .collect();
                    for pid in pids {
                        let upid = pid as usize;
                        if !current_running_workers.contains(&&upid) {
                            println!("Will remove worker #{} which is dead", pid);
                            Self::remove_running_worker(running_workers.clone(), pid);

                            // another check before start new workers, because it might be
                            // close here when stop signal was triggered
                            if !stop_signal.load(Ordering::SeqCst) {
                                println!("Will start another worker to fill in");
                                let child = Self::start_worker(
                                    &worker_binpath,
                                    &worker_outpath,
                                    &worker_timeout,
                                )
                                .expect("failed to start worker");
                                Self::add_running_worker(running_workers.clone(), child);
                            }
                        }
                    }
                }
                println!("<-- [watch_workers]");
            }
        });
        Ok(())
    }

    pub fn send_stop_signal(graceful_time: u64) -> Result<()> {
        println!("Will give workers time to gracefully shutdown");
        thread::sleep(Duration::from_secs(graceful_time));

        println!("Will terminate socket now");
        Self::send_stop_signal_to_control_socket()
    }

    fn send_stop_signal_to_control_socket() -> Result<()> {
        let ctx = zmq::Context::new();
        let control = ctx.socket(zmq::PUB).unwrap();
        control
            .bind("tcp://127.0.0.1:6662")
            .expect("failed connecting control");
        thread::sleep(Duration::from_secs(1));

        match control.send("TERMINATE", 0) {
            Ok(()) => Ok(()),
            Err(reason) => Err(AnyError::from(reason)),
        }
    }

    fn stop(&self) -> Result<()> {
        // TODO: not sure exactly what to do here now
        Ok(())
    }
}

// Unit testing
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_broker() {
        let broker = Broker::new(
            0,
            Arc::new(AtomicBool::new(false)),
            2,
            Path::new("bin"),
            Path::new("out"),
            Duration::from_secs(5),
        );
        assert_eq!(broker.worker_instances, 2);
        assert_eq!(broker.worker_binpath.as_os_str(), "bin");
        assert_eq!(broker.worker_outpath.as_os_str(), "out");
    }

    #[test]
    fn start_broker() {
        assert!(true);
    }

    #[test]
    fn stop_broker() {
        assert!(true);
    }
}

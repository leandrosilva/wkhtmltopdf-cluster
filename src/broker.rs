use super::error::{AnyError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::{thread, time};
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
    pub worker_instances: usize,
    pub worker_binpath: PathBuf,
    pub worker_outpath: PathBuf,
    running_workers: HashMap<u32, WorkerRef>,
}

impl Broker {
    pub fn new(
        id: u32,
        worker_instances: usize,
        worker_binpath: &Path,
        worker_outpath: &Path,
    ) -> Broker {
        let instance = Broker {
            id: id,
            worker_instances: worker_instances,
            worker_binpath: PathBuf::from(worker_binpath),
            worker_outpath: PathBuf::from(worker_outpath),
            running_workers: HashMap::new(),
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
            let pids = self.running_workers.iter().map(|kv| *kv.0).collect();
            // workers are ready, so notify it
            on_ready(pids);
        })
        .expect("failed to start proxy");
        Ok(())
    }

    fn start_workers(&mut self) -> Result<()> {
        for i in 0..self.worker_instances {
            let child = Command::new(&self.worker_binpath)
                .arg("start")
                .arg("--output")
                .arg(&self.worker_outpath.to_str().unwrap())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect(format!("failed to start #{} {:?}", i, self.worker_binpath).as_str());
            self.running_workers.insert(
                child.id(),
                WorkerRef {
                    pid: child.id(),
                    os_process: child,
                },
            );
            thread::sleep(time::Duration::from_millis(100));
        }
        Ok(())
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

        std::thread::spawn(move || loop {
            println!("-->");
            let sys = System::new_all();
            for (pid, process) in sys.get_processes() {
                if process.name().contains(worker_exec.as_str()) {
                    if let Some(parent) = process.parent() {
                        if parent == id {
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
            println!("<--");
            thread::sleep(Duration::from_secs(5));
        });
        Ok(())
    }

    pub fn send_stop_signal(worker_instances: usize, graceful_time: u64) -> Result<()> {
        let ctx = zmq::Context::new();

        println!("Will send STOP to {} worker(s)", graceful_time);
        Self::send_stop_signal_to_workers(&ctx, worker_instances)?;

        println!("Will give them time to gracefully shutdown");
        thread::sleep(Duration::from_secs(graceful_time));

        println!("Will terminate socket now");
        Self::send_stop_signal_to_control_socket(&ctx)
    }

    fn send_stop_signal_to_workers(ctx: &zmq::Context, worker_instances: usize) -> Result<()> {
        let stop_cmd = ctx.socket(zmq::REQ).unwrap();
        stop_cmd
            .connect("tcp://127.0.0.1:6660")
            .expect("failed connecting as client");
        thread::sleep(Duration::from_secs(1));

        for i in 0..worker_instances {
            if let Err(reason) = stop_cmd.send("STOP", 0) {
                return Err(AnyError::from(reason));
            };
            let resp = stop_cmd
                .recv_string(0)
                .expect("failed to receive STOP response")
                .expect("could not parse STOP response");
            println!("Sent STOP to worker ({}) and got: {}", i + 1, resp);
        }
        Ok(())
    }

    fn send_stop_signal_to_control_socket(ctx: &zmq::Context) -> Result<()> {
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
        let broker = Broker::new(0, 2, Path::new("bin"), Path::new("out"));
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

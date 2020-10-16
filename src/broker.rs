use super::error::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;
use std::{thread, time};
use zmq;

#[derive(Debug)]
struct WorkerRef {
    pid: u32,
    os_process: Child,
}

#[derive(Debug)]
pub struct Broker {
    pub worker_instances: usize,
    pub worker_binpath: PathBuf,
    pub worker_outpath: PathBuf,
    running_workers: HashMap<u32, WorkerRef>,
}

impl Broker {
    pub fn new(worker_instances: usize, worker_binpath: &Path, worker_outpath: &Path) -> Broker {
        let instance = Broker {
            worker_instances: worker_instances,
            worker_binpath: PathBuf::from(worker_binpath),
            worker_outpath: PathBuf::from(worker_outpath),
            running_workers: HashMap::new(),
        };
        instance
    }

    pub fn start<F: Fn(Vec<u32>)>(&mut self, on_ready: F) -> Result<()> {
        self.start_workers().expect("failed to start workers");
        start_proxy(|| {
            let pids = self.running_workers.iter().map(|kv| *kv.0).collect();
            on_ready(pids);
        });
        Ok(())
    }

    pub fn send_stop_signal() -> Result<()> {
        let ctx = zmq::Context::new();
        let control = ctx.socket(zmq::PUB).unwrap();
        control
            .bind("tcp://127.0.0.1:6662")
            .expect("failed connecting control");
        thread::sleep(Duration::from_secs(1));

        control
            .send("TERMINATE", 0)
            .expect("failed to send TERMINATE message");
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        // TODO: block requests, finish work in process, stop proxy and then workers
        println!("Stopping...");
        Ok(())
    }

    fn start_workers(&mut self) -> Result<()> {
        for i in 0..self.worker_instances {
            let child = Command::new(&self.worker_binpath)
                .arg("start")
                .arg("--output")
                .arg(&self.worker_outpath.to_str().unwrap())
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
}

fn start_proxy<F: Fn()>(on_ready: F) {
    let ctx = zmq::Context::new();

    // Socket facing clients
    let mut frontend = ctx.socket(zmq::ROUTER).unwrap();
    frontend
        .bind("tcp://127.0.0.1:6660")
        .expect("failed connecting frontend");

    // Socket facing workers
    let mut backend = ctx.socket(zmq::DEALER).unwrap();
    backend
        .bind("tcp://127.0.0.1:6661")
        .expect("failed connecting backend");

    // Socket for controlling
    let mut control = ctx.socket(zmq::SUB).unwrap();
    control
        .connect("tcp://127.0.0.1:6662")
        .expect("failed connecting control");
    control
        .set_subscribe(b"")
        .expect("failed subscribing to control");

    on_ready();
    zmq::proxy_steerable(&mut frontend, &mut backend, &mut control).expect("failed to start proxy");
}

// Unit testing
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_broker() {
        let broker = Broker::new(2, Path::new("bin"), Path::new("out"));
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

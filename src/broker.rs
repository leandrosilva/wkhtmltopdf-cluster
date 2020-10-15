use super::error::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
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

    pub fn start(&mut self) -> Result<()> {
        match self.start_workers() {
            Ok(()) => {
                println!("Workers are ready to rock now!");
                for (pid, _worker) in &self.running_workers {
                    println!("- Worker PID: {}", pid);
                }
            }
            Err(reason) => eprintln!("Failed due to: {}", reason),
        }
        start_proxy();
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        // TODO: block requests, finish work in process, stop proxy and then workers
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

fn start_proxy() {
    let ctx = zmq::Context::new();

    // Socket facing clients
    let frontend = ctx.socket(zmq::ROUTER).unwrap();
    frontend
        .bind("tcp://127.0.0.1:9999")
        .expect("failed connecting frontend");

    // Socket facing workers
    let backend = ctx.socket(zmq::DEALER).unwrap();
    backend
        .bind("tcp://127.0.0.1:6666")
        .expect("failed connecting backend");

    zmq::proxy(&frontend, &backend).expect("failed to start proxying");
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

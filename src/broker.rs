use error_chain::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{thread, time};
use zmq;

error_chain! {}

#[derive(Debug)]
pub struct Broker {
    pub worker_instances: usize,
    pub worker_binpath: PathBuf,
    pub worker_outpath: PathBuf,
}

impl Broker {
    pub fn new(worker_instances: usize, worker_binpath: &Path, worker_outpath: &Path) -> Broker {
        let instance = Broker {
            worker_instances: worker_instances,
            worker_binpath: PathBuf::from(worker_binpath),
            worker_outpath: PathBuf::from(worker_outpath),
        };
        instance
    }

    pub fn start(&self) -> Result<()> {
        match start_workers(
            self.worker_instances,
            &self.worker_binpath,
            &self.worker_outpath,
        ) {
            Ok(()) => println!("Workers are ready to rock!"),
            Err(reason) => eprintln!("Failed due to: {}", reason),
        }
        start_proxy();
        Ok(())
    }
}

fn start_workers(w_instances: usize, w_binpath: &Path, w_output: &Path) -> Result<()> {
    for i in 0..w_instances {
        let child = Command::new(w_binpath)
            .arg("start")
            .arg("--output")
            .arg(w_output.to_str().unwrap())
            .spawn()
            .expect(format!("failed to start #{} {:?}", i, w_binpath).as_str());
        println!("PID: {}", child.id());
        thread::sleep(time::Duration::from_millis(100));
    }

    Ok(())
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
}

use super::error::{AnyError, Result};
use super::helpers::{zmq_assert_empty, zmq_recv_string, zmq_send_multipart};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use sysinfo::{Process, ProcessExt, Signal, System, SystemExt};
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
        self.run_eventloop(|| {
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
            Self::register_worker(self.running_workers.clone(), child);
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

    fn register_worker(running_workers: Arc<RwLock<HashMap<u32, WorkerRef>>>, child: Child) {
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

    fn remove_worker(running_workers: Arc<RwLock<HashMap<u32, WorkerRef>>>, pid: &u32) {
        running_workers
            .write()
            .expect("failed to acquire write lock to remove dead worker")
            .remove(&pid);
    }

    fn run_eventloop<F: Fn()>(&self, on_ready: F) -> Result<()> {
        let context = zmq::Context::new();

        let frontend_socket = context.socket(zmq::ROUTER).unwrap();
        frontend_socket
            .bind("tcp://127.0.0.1:6660")
            .expect("failed binding frontend socket");

        let backend_socket = context.socket(zmq::ROUTER).unwrap();
        backend_socket
            .bind("tcp://127.0.0.1:6661")
            .expect("failed binding backend socket");

        println!(
            "Listening on:\n- Frontend: tcp://127.0.0.1:6660\n- Backend: tcp://127.0.0.1:6661"
        );

        // ready to start proxying, so notify it
        on_ready();

        let mut available_workers = VecDeque::new();

        while !self.stop_signal.load(Ordering::SeqCst) {
            let mut service_sockets = [
                backend_socket.as_poll_item(zmq::POLLIN),
                frontend_socket.as_poll_item(zmq::POLLIN),
            ];

            // should only poll frontend if there is backend ready to work
            let target_sockets = if available_workers.is_empty() { 1 } else { 2 };

            let poll_result = zmq::poll(&mut service_sockets[0..target_sockets], 1000)
                .expect("failed to poll sockets");
            if poll_result == -1 {
                println!("Will STOP due to error polling sockets");
                self.stop_signal.store(true, Ordering::SeqCst);
            }

            // -- backend
            if service_sockets[0].is_readable() {
                self.handle_backend_talking(
                    &backend_socket,
                    &frontend_socket,
                    &mut available_workers,
                )
                .expect("failed handling backend worker");
            }

            // -- frontend
            if service_sockets[1].is_readable() {
                self.handle_frontend_talking(
                    &backend_socket,
                    &frontend_socket,
                    &mut available_workers,
                )
                .expect("failed handling frontend client");
            }
        }

        println!("Will stop listening on sockets");
        Ok(())
    }

    fn handle_backend_talking(
        &self,
        backend_socket: &zmq::Socket,
        frontend_socket: &zmq::Socket,
        available_workers: &mut VecDeque<String>,
    ) -> Result<()> {
        // worker envelope:
        //   ID>, EMPTY, READY
        //   ID>, EMPTY, GONE
        //   ID>, EMPTY, CLIENT, EMPTY, REPLY
        //   ID>, EMPTY, CLIENT, EMPTY, REPLY, EMPTY, CONTENT

        let worker_id = zmq_recv_string(&backend_socket, "failed reading ID of worker's envelope");
        available_workers.push_front(worker_id.clone());

        zmq_assert_empty(
            &backend_socket,
            "failed reading 1st <EMPTY> of worker's envelope",
        );

        let worker_message = zmq_recv_string(
            &backend_socket,
            "failed reading <MESSAGE> of worker's envelope",
        );

        match worker_message.as_str() {
            "READY" => println!("Worker #{} is ready", worker_id),
            "GONE" => println!("Worker #{} is gone", worker_id),
            client_id => {
                zmq_assert_empty(
                    &backend_socket,
                    "failed reading 2nd <EMPTY> of worker's envelope",
                );
                let reply = zmq_recv_string(
                    &backend_socket,
                    "failed reading <REPLY> of worker's envelope",
                );
                zmq_assert_empty(
                    &backend_socket,
                    "failed reading 3nd <EMPTY> of worker's envelope",
                );
                let content = zmq_recv_string(
                    &backend_socket,
                    "failed reading <CONTENT> of worker's envelope",
                );
                println!(
                    "Worker #{} send reply {} to client #{}: {}",
                    worker_id, reply, client_id, content
                );

                // multipart envelope from worker to client:
                //   CLIENT, EMPTY, WORKER, EMPTY, REPLY, EMPTY, CONTENT
                let reply_envelope = vec![
                    client_id.as_bytes().to_vec(),
                    b"".to_vec(),
                    worker_id.as_bytes().to_vec(),
                    b"".to_vec(),
                    reply.as_bytes().to_vec(),
                    b"".to_vec(),
                    content.as_bytes().to_vec(),
                ];

                // forward reply envelope to given client
                zmq_send_multipart(
                    &frontend_socket,
                    reply_envelope,
                    format!(
                        "failed forwarding reply from worker #{} to client #{}",
                        worker_id, client_id
                    )
                    .as_str(),
                );
            }
        }
        Ok(())
    }

    fn handle_frontend_talking(
        &self,
        backend_socket: &zmq::Socket,
        frontend_socket: &zmq::Socket,
        available_workers: &mut VecDeque<String>,
    ) -> Result<()> {
        // client envelope:
        //   ID, EMPTY, REQUEST

        let client_id = zmq_recv_string(
            &frontend_socket,
            "failed reading <ID> from client's envelope",
        );
        zmq_assert_empty(
            &frontend_socket,
            "failed reading 1nd <EMPTY> of client's envelope",
        );
        let request = zmq_recv_string(
            &frontend_socket,
            "failed reading <REQUEST> from client's envelope",
        );

        println!("Current available workers: {:?}", available_workers);
        let worker_id = available_workers
            .pop_back()
            .expect("failed to get an available worker");

        // multipart envelope from client to worker:
        //   WORKER, EMPTY, CLIENT, EMPTY, REQUEST
        let reply_envelope = vec![
            worker_id.as_bytes().to_vec(),
            "".as_bytes().to_vec(),
            client_id.as_bytes().to_vec(),
            "".as_bytes().to_vec(),
            request.as_bytes().to_vec(),
        ];

        // forward request envelope to given worker
        zmq_send_multipart(
            &backend_socket,
            reply_envelope,
            format!(
                "failed forwarding request from client #{} to worker #{}",
                client_id, worker_id
            )
            .as_str(),
        );
        Ok(())
    }

    fn watch_workers(&self) -> Result<()> {
        let id = self.id;
        let stop_signal = self.stop_signal.clone();
        let worker_binpath = self.worker_binpath.clone();
        let worker_outpath = self.worker_outpath.clone();
        let worker_timeout = self.worker_timeout;
        let running_workers = self.running_workers.clone();

        std::thread::spawn(move || {
            while !stop_signal.load(Ordering::SeqCst) {
                // it's better to wait at start then at end, because at the end
                // the sleeping might interfere on the shutting down process
                thread::sleep(Duration::from_secs(5)); // TODO: parametize it

                println!("--> [watch_workers]");
                // actual workers running now under this broker
                let current_running_workers =
                    Self::get_current_running_workers(&id, |parent_id, pid, process| {
                        println!(
                            "[{}:{}] {} {}kB {}%",
                            parent_id,
                            pid,
                            process.name(),
                            process.memory(),
                            process.cpu_usage()
                        );
                    });

                // house keeping
                if !stop_signal.load(Ordering::SeqCst) {
                    let pids: Vec<u32> = running_workers
                        .read()
                        .expect("failed to acquire lock of running workers")
                        .iter()
                        .map(|kv| *kv.0)
                        .collect();
                    for pid in pids {
                        if !current_running_workers.contains(&pid) {
                            println!("Will remove worker #{} which is dead", pid);
                            Self::remove_worker(running_workers.clone(), &pid);

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
                                Self::register_worker(running_workers.clone(), child);
                            }
                        }
                    }
                }
                println!("<-- [watch_workers]");
            }
        });
        Ok(())
    }

    fn get_current_running_workers<F: Fn(u32, u32, &Process)>(id: &u32, callback: F) -> Vec<u32> {
        let mut worker_processes: Vec<u32> = Vec::new();
        let sys = System::new_all();
        for (pid, process) in sys.get_processes() {
            if let Some(parent_id) = process.parent() {
                if parent_id == *id as usize {
                    worker_processes.push(*pid as u32);
                    callback(parent_id as u32, *pid as u32, process);
                }
            }
        }
        worker_processes
    }

    fn stop(&self) -> Result<()> {
        self.terminate_workers(5); // TODO: parametize it
        println!("Everything ready to stop");
        Ok(())
    }

    fn terminate_workers(&self, graceful_time: u64) {
        println!("Will give workers time to gracefully shutdown");
        thread::sleep(Duration::from_secs(graceful_time));

        println!("Will ensure all workers terminate one way or another");
        Self::get_current_running_workers(&self.id, |_, pid, process| {
            println!("Worker #{} still running and will be terminated", pid);
            process.kill(Signal::Kill);
        });
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

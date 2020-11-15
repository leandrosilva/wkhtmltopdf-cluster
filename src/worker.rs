use super::error::Result;
use super::helpers::{zmq_assert_empty, zmq_recv_string, zmq_send, zmq_send_multipart};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use wkhtmltopdf::{Orientation, PdfApplication, Size};
use zmq;

#[derive(Debug)]
pub struct Worker {
    id: u32,
    stop_signal: Arc<AtomicBool>,
    output_dir: PathBuf,
    timeout: Duration,
}

impl Worker {
    pub fn new(
        id: u32,
        stop_signal: Arc<AtomicBool>,
        output_dir: &Path,
        timeout: Duration,
    ) -> Worker {
        let instance = Worker {
            id: id,
            stop_signal: stop_signal,
            output_dir: PathBuf::from(output_dir),
            timeout: timeout,
        };
        instance
    }

    pub fn run<'a, F: 'a + Fn()>(&'a mut self, on_ready: F) -> Result<()> {
        let (heartbeat_tx, heartbeat_rx) = channel::<()>();
        self.watch_eventloop(heartbeat_rx);
        self.run_eventloop(heartbeat_tx, on_ready)
    }

    fn watch_eventloop(&self, heartbeat_rx: Receiver<()>) {
        let id = self.id;
        let timeout = self.timeout;
        let stop_signal = self.stop_signal.clone();

        thread::spawn(move || {
            while !stop_signal.load(Ordering::SeqCst) {
                if heartbeat_rx.recv_timeout(timeout).is_err() {
                    println!(
                        "[#{}] Hugging for more than {}s and will terminate now",
                        id,
                        timeout.as_secs()
                    );
                    process::exit(666);
                }
            }
        });
    }

    fn run_eventloop<'a, F: 'a + Fn()>(
        &'a mut self,
        heartbeat_tx: Sender<()>,
        on_ready: F,
    ) -> Result<()> {
        let socket_id = format!("W{}", self.id);
        let context = zmq::Context::new();
        let service_socket = context.socket(zmq::REQ).unwrap();
        service_socket.set_identity(socket_id.as_bytes())?;
        service_socket.set_sndtimeo(1000)?;
        service_socket.set_rcvtimeo(1000)?;
        service_socket
            .connect("tcp://127.0.0.1:6661")
            .expect("failed listening on port 6661");

        // Enclosing scope for PdfApplication
        {
            let mut pdf_app = PdfApplication::new().expect("failed to init PDF application");

            // worker is ready, so notify it
            on_ready();
            zmq_send(&service_socket, "READY", "failed sending <READY> to broker");

            while !self.stop_signal.load(Ordering::SeqCst) {
                // send heartbeat to signal it's not hugging
                if let Err(reason) = heartbeat_tx.send(()) {
                    println!(
                        "[#{}] Something went weird with the heartbeat monitoring: {:?}",
                        self.id, reason
                    );
                    break;
                }

                // try to grap some work to do...
                let message = match service_socket.recv_string(0) {
                    Ok(received) => received.unwrap(),
                    Err(_) => continue,
                };
                println!("[#{}] Message: {}", self.id, message);

                // -- from broker
                if message == "STOP" {
                    zmq_send(&service_socket, "GONE", "failed to send <GONE> response");
                    break;
                }

                // -- from client
                let client_id = message;
                zmq_assert_empty(
                    &service_socket,
                    "failed reading 1nd <EMPTY> of client's envelope",
                );
                let request = zmq_recv_string(
                    &service_socket,
                    "failed reading <REQUEST> from client's envelope",
                );
                println!("[#{}] Client #{} request: {}", self.id, client_id, request);

                let payload = request; // TODO: to be JSON
                println!("[#{}] Client #{} payload: {}", self.id, client_id, payload);

                let message_id = get_uid();
                let url = match payload.parse() {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        let err_msg = format!("Cannot parse URL: {}", payload);
                        println!("{}", err_msg.as_str());

                        // build reply multipart envelope to client as:
                        // CLIENT, EMPTY, REPLY, EMPTY, CONTENT
                        let reply_envelope = vec![
                            client_id.as_bytes().to_vec(),
                            b"".to_vec(),
                            b"ERROR".to_vec(),
                            b"".to_vec(),
                            err_msg.as_bytes().to_vec(),
                        ];

                        zmq_send_multipart(
                            &service_socket,
                            reply_envelope,
                            format!("failed sending reply to client #{}", client_id).as_str(),
                        );
                        continue;
                    }
                };
                let filepath = self.output_dir.join(Path::new(
                    format!("req-{}-{}.pdf", self.id, message_id).as_str(),
                ));
                unsafe {
                    let mut pdfout = pdf_app
                        .builder()
                        .orientation(Orientation::Landscape)
                        .margin(Size::Inches(2))
                        .title("A taste of WkHTMLtoPDF Cluster")
                        .object_setting("load.windowStatus", "ready")
                        .build_from_url(url)
                        .expect(format!("failed to build {}", filepath.to_str().unwrap()).as_str());
                    pdfout
                        .save(&filepath)
                        .expect(format!("failed to save {}", filepath.to_str().unwrap()).as_str());
                }
                println!(
                    "[#{}] Built PDF is saved as: {}",
                    self.id,
                    filepath.to_str().unwrap()
                );

                let content = format!("PDF saved at output directory");

                // build reply multipart envelope to client as:
                // CLIENT, EMPTY, REPLY, EMPTY, CONTENT
                let reply_envelope = vec![
                    client_id.as_bytes().to_vec(),
                    b"".to_vec(),
                    b"REPLY".to_vec(),
                    b"".to_vec(),
                    content.as_bytes().to_vec(),
                ];

                zmq_send_multipart(
                    &service_socket,
                    reply_envelope,
                    format!("failed sending reply to client #{}", client_id).as_str(),
                );
            }
        }

        println!("[#{}] Stopping...", self.id);
        service_socket
            .disconnect("tcp://127.0.0.1:6661")
            .expect("failed disconnecting on port 6661");
        println!("[#{}] Disconnected from broker to stop", self.id);

        Ok(())
    }
}

pub fn get_uid() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");
    let time_in_ms =
        since_the_epoch.as_secs() * 1000 + since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    time_in_ms
}

// Unit testing
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_worker() {
        let worker = Worker::new(
            123,
            Arc::new(AtomicBool::new(false)),
            Path::new("out"),
            Duration::from_secs(3),
        );
        assert_eq!(worker.id, 123);
        assert_eq!(worker.output_dir.as_os_str(), "out");
    }

    #[test]
    fn start_worker() {
        assert!(true);
    }

    #[test]
    fn stop_worker() {
        assert!(true);
    }
}

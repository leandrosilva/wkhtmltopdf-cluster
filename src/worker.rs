use super::error::Result;
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
        println!(">>> {} before event loop", self.id);
        // heartbeat monitoring setup
        let local_id = self.id;
        let local_timeout = self.timeout;
        let local_stop_signal = self.stop_signal.clone();
        let (heartbeat_tx, heartbeat_rx) = channel::<()>();
        thread::spawn(move || {
            Self::heartbeat_monitoring(local_id, local_stop_signal, heartbeat_rx, local_timeout);
        });

        // loop 'til close to forever or whatever
        let result = self.run_eventloop(heartbeat_tx, on_ready);
        println!("<<< {} after event loop", self.id);
        result
    }

    fn heartbeat_monitoring(
        id: u32,
        stop_signal: Arc<AtomicBool>,
        heartbeat_rx: Receiver<()>,
        timeout: Duration,
    ) {
        while !stop_signal.load(Ordering::SeqCst) {
            if heartbeat_rx.recv_timeout(timeout).is_err() {
                println!(
                    "[#{}] Worker is hugging for more than {}s and will be killed now",
                    id,
                    timeout.as_secs()
                );
                process::exit(666);
            }
        }
    }

    fn run_eventloop<'a, F: 'a + Fn()>(
        &'a mut self,
        heartbeat_tx: Sender<()>,
        on_ready: F,
    ) -> Result<()> {
        let ctx = zmq::Context::new();
        let service_socket = ctx.socket(zmq::REP).unwrap();
        service_socket
            .connect("tcp://127.0.0.1:6661")
            .expect("failed listening on port 6661");
        service_socket.set_rcvtimeo(1000)?;
        service_socket.set_sndtimeo(1000)?;

        // Enclosing scope for PdfApplication
        {
            let mut pdf_app = PdfApplication::new().expect("failed to init PDF application");

            // worker is ready, so notify it
            on_ready();

            while !self.stop_signal.load(Ordering::SeqCst) {
                // send heartbeat to signal it's not hugging
                if let Err(reason) = heartbeat_tx.send(()) {
                    println!(
                        "[#{}] Something went weird with heartbeat monitoring: {:?}",
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

                if message.to_uppercase() == "STOP" {
                    service_socket
                        .send(format!("[#{}] Shutting down", self.id).as_str(), 0)
                        .expect("failed to send STOP response");
                    break;
                }

                let message_id = get_uid();
                let url = match message.parse() {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        let err_msg = format!("[#{}] Cannot parse URL: {}", self.id, message);
                        println!("{}", err_msg.as_str());
                        service_socket
                            .send(format!("{}", err_msg.as_str()).as_str(), 0)
                            .expect("failed sending message");
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
                service_socket
                    .send(
                        format!("[#{}] PDF saved at output directory", self.id).as_str(),
                        0,
                    )
                    .expect("failed sending message");
            }
        }

        println!("[#{}] Stopping...", self.id);
        service_socket
            .disconnect("tcp://127.0.0.1:6661")
            .expect("failed disconnecting on port 6661");
        println!("[#{}] Disconnected from broker", self.id);

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
        let stop_signal = Arc::new(AtomicBool::new(false));
        let worker = Worker::new(123, stop_signal.clone(), Path::new("out"));
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

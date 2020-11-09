use super::error::Result;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use wkhtmltopdf::{Orientation, PdfApplication, Size};
use zmq;

#[derive(Debug)]
pub struct Worker {
    id: u32,
    stop_signal: Arc<AtomicBool>,
    output_dir: PathBuf,
}

pub struct EventLoopInput {
    worker: Arc<RwLock<Worker>>,
    on_ready: Arc<RwLock<dyn Fn() + Send + Sync>>,
}

unsafe impl Send for Worker {}

impl Worker {
    pub fn new(id: u32, stop_signal: Arc<AtomicBool>, output_dir: &Path) -> Worker {
        let instance = Worker {
            id: id,
            stop_signal: stop_signal,
            output_dir: PathBuf::from(output_dir),
        };
        instance
    }

    pub fn run_worker<F: 'static + Fn() + Send + Sync>(this: Self, on_ready: F) -> Result<()> {
        let id = this.id;
        println!(">>> {} before event loop", id);
        let input = EventLoopInput {
            worker: Arc::new(RwLock::new(this)),
            on_ready: Arc::new(RwLock::new(on_ready)),
        };
        let result = Self::run_worker_eventloop(&input);
        println!("<<< {} after event loop", id);
        result
    }

    fn run_worker_eventloop(input: &EventLoopInput) -> Result<()> {
        let local_worker = input.worker.clone();
        let local_on_ready = input.on_ready.clone();
        let event_loop_handle = thread::spawn(move || {
            let mut worker = local_worker
                .write()
                .expect("faild to acquire writing lock of worker");
            let on_ready = local_on_ready
                .read()
                .expect("failed to acquire read lock on on_ready callback");
            worker.run_eventloop(on_ready.deref())
        });
        let result = event_loop_handle
            .join()
            .expect("failed to join event loop thread");
        result
    }

    pub fn run<'a, F: 'a + Fn()>(&'a mut self, on_ready: F) -> Result<()> {
        println!(">>> {} before event loop", self.id);
        let result = self.run_eventloop(on_ready);
        println!("<<< {} after event loop", self.id);
        result
    }

    fn run_eventloop<'a, F: 'a + Fn()>(&'a mut self, on_ready: F) -> Result<()> {
        let ctx = zmq::Context::new();
        let subscriber = ctx.socket(zmq::REP).unwrap();
        subscriber
            .connect("tcp://127.0.0.1:6661")
            .expect("failed listening on port 6661");
        subscriber.set_rcvtimeo(1000)?;
        subscriber.set_sndtimeo(1000)?;

        // Enclosing scope for PdfApplication
        {
            let mut pdf_app = PdfApplication::new().expect("failed to init PDF application");

            // worker is ready, so notify it
            on_ready();

            loop {
                // got stop signal?
                let should_stop = self.stop_signal.load(Ordering::SeqCst);
                if should_stop {
                    break;
                }

                // grap some work to do...
                let message = match subscriber.recv_string(0) {
                    Ok(received) => received.unwrap(),
                    Err(_) => continue,
                };
                println!("[#{}] Message: {}", self.id, message);

                if message.to_uppercase() == "STOP" {
                    subscriber
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
                        subscriber
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
                subscriber
                    .send(
                        format!("[#{}] PDF saved at output directory", self.id).as_str(),
                        0,
                    )
                    .expect("failed sending message");
            }
        }

        println!("[#{}] Stopping...", self.id);
        subscriber
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

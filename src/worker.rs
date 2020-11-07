use super::error::Result;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use wkhtmltopdf::{Orientation, PdfApplication, Size};
use zmq;

#[derive(Debug)]
pub struct Worker {
    id: u32,
    output_dir: PathBuf,
}

impl Worker {
    pub fn new(id: u32, output_dir: &Path) -> Worker {
        let instance = Worker {
            id: id,
            output_dir: PathBuf::from(output_dir),
        };
        instance
    }

    pub fn run<F: Fn()>(&mut self, on_ready: F) -> Result<()> {
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
                println!("<<keep running?>>");

                // grap some work to do...               
                let message = match subscriber.recv_string(0) {
                    Ok(received) => received.unwrap(),
                    Err(_) => continue
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
                let filepath = self
                    .output_dir
                    .join(Path::new(format!("google-{}.pdf", message_id).as_str()));
                let mut pdfout = pdf_app
                    .builder()
                    .orientation(Orientation::Landscape)
                    .margin(Size::Inches(2))
                    .title("A taste of WkHTMLtoPDF Cluster")
                    .build_from_url(url)
                    .expect(format!("failed to build {}", filepath.to_str().unwrap()).as_str());
                pdfout
                    .save(&filepath)
                    .expect(format!("failed to save {}", filepath.to_str().unwrap()).as_str());
                println!("Generated PDF saved as: {}", filepath.to_str().unwrap());
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
        let worker = Worker::new(123, Path::new("out"));
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

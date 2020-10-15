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

    pub fn start(&mut self) -> Result<()> {
        let ctx = zmq::Context::new();
        let subscriber = ctx.socket(zmq::REP).unwrap();
        subscriber
            .connect("tcp://127.0.0.1:6666")
            .expect("failed to listen on port 6666");
        let mut pdf_app = PdfApplication::new().expect("failed to init PDF application");
        loop {
            let message = subscriber
                .recv_string(0)
                .expect("failed receiving message")
                .unwrap();
            println!("[#{}] Message: {}", self.id, message);
            if message == "END" {
                break;
            }
            let message_id = get_uid();
            let url = message;
            let filepath = self
                .output_dir
                .join(Path::new(format!("google-{}.pdf", message_id).as_str()));
            let mut pdfout = pdf_app
                .builder()
                .orientation(Orientation::Landscape)
                .margin(Size::Inches(2))
                .title("A taste of WkHTMLtoPDF Cluster")
                .build_from_url(url.parse().unwrap())
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
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        // TODO: whatever it takes
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

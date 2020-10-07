use clap::{App, Arg};
use error_chain::*;
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use wkhtmltopdf::{Orientation, PdfApplication, Size};

error_chain! {}

// $ cargo run -p wkhtmltopdf-cluster --bin worker start --output ./examples/pdf
//

fn main() {
    let mut app = App::new("WkHTMLtoPDF Cluster")
        .version("1.0")
        .author("Leandro Silva <leandrodoze@gmail.com>")
        .about("This is the worker node process.")
        .subcommand(
            App::new("start")
                .about("Starts the worker node for a given cluster")
                .arg(
                    Arg::with_name("output")
                        .about("output directory")
                        .short('o')
                        .long("output")
                        .takes_value(true)
                        .value_name("DIR")
                        .required(true),
                ),
        );

    let matches = app.get_matches_mut();
    match matches.subcommand() {
        ("start", Some(sub_matches)) => {
            let output_dir = Path::new(sub_matches.value_of("output").unwrap());

            let worker_id = get_pid();

            println!("WkHTMLtoPDF Cluster :: Worker :: Start [#{}]", worker_id);
            match run(worker_id, output_dir) {
                Ok(()) => println!("Work is done!"),
                Err(reason) => eprintln!("Failed due to: {}", reason),
            }
            println!("WkHTMLtoPDF Cluster :: Worker :: End [#{}]", worker_id);
        }
        ("", None) => app.print_help().unwrap(),
        _ => unreachable!(),
    }
}

fn run(worker_id: u32, output_dir: &Path) -> Result<()> {
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
        println!("[#{}] Message: {}", worker_id, message);
        if message == "END" {
            break;
        }

        let message_id = get_uid();
        let url = message;
        let filepath = output_dir.join(Path::new(format!("google-{}.pdf", message_id).as_str()));

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
            .send(format!("[#{}] PDF saved at output directory", worker_id).as_str(), 0)
            .expect("failed sending message");
    }

    Ok(())
}

fn get_pid() -> u32 {
    process::id()
}

fn get_uid() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");
    let time_in_ms =
        since_the_epoch.as_secs() * 1000 + since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    time_in_ms
}

use clap::{App, Arg};
use std::path::Path;
use wkhtmltopdf::{Orientation, PdfApplication, Size};
use error_chain::*;

error_chain! {}

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

            println!("WkHTMLtoPDF Cluster :: Worker :: Start");
            start_subscriber();
            match run(output_dir) {
                Ok(()) => println!("Work done"),
                Err(reason) => eprintln!("Failed due to: {}", reason),
            }
            println!("WkHTMLtoPDF Cluster :: Worker :: End");
        }
        ("", None) => app.print_help().unwrap(),
        _ => unreachable!(),
    }
}

fn start_subscriber() {
    let ctx = zmq::Context::new();

    let subscriber = ctx.socket(zmq::PUB).unwrap();
    subscriber.connect("tcp://127.0.0.1:6666").expect("failed to listen on port 6666");
}

fn run(output_dir: &Path) -> Result<()> {
    let url = "https://www.google.com.br";
    let mut pdf_app = PdfApplication::new().expect("failed to init PDF application");

    for i in 0..3 {
        let filepath = output_dir.join(Path::new(format!("google-{}.pdf", i).as_str()));

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
    }

    Ok(())
}

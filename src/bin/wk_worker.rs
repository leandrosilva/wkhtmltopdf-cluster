use clap::{App, Arg};
use error_chain::*;
use std::fs;
use std::process;
use std::path::Path;
use wkhtmltopdf_cluster::worker::Worker;

error_chain! {
    foreign_links {
        Io(std::io::Error);
    }
}

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
            create_keys_dir_if_not_exists(&output_dir).unwrap();

            let worker_id = process::id();

            println!("WkHTMLtoPDF Cluster :: Worker :: Start [#{}]", worker_id);
            let mut worker = Worker::new(worker_id, output_dir);
            worker.start().unwrap();
            println!("WkHTMLtoPDF Cluster :: Worker :: End [#{}]", worker_id);
        }
        ("", None) => app.print_help().unwrap(),
        _ => unreachable!(),
    }
}

pub fn create_keys_dir_if_not_exists(output_dir: &Path) -> Result<()> {
    if output_dir.is_dir() {
        return Ok(());
    }

    Ok(fs::create_dir_all(output_dir).unwrap())
}

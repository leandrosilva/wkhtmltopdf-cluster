use clap::{App, Arg};
use std::path::Path;
use std::process;
use wkhtmltopdf_cluster::helpers::create_dir_if_not_exists;
use wkhtmltopdf_cluster::worker::Worker;

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

    watch_stop_signal();

    let matches = app.get_matches_mut();
    match matches.subcommand() {
        ("start", Some(sub_matches)) => {
            let output_dir = Path::new(sub_matches.value_of("output").unwrap());
            create_dir_if_not_exists(&output_dir).expect("failed to create directory");

            let worker_id = process::id();

            println!("WkHTMLtoPDF Cluster :: Worker :: Start [#{}]", worker_id);
            let mut worker = Worker::new(worker_id, output_dir);
            worker
                .run(|| println!("- Worker #{} is ready", worker_id))
                .expect("failed running worker");
            println!("WkHTMLtoPDF Cluster :: Worker :: End [#{}]", worker_id);
            process::exit(0);
        }
        ("", None) => app.print_help().unwrap(),
        _ => unreachable!(),
    }
}

fn watch_stop_signal() {
    let worker_id = process::id();
    let mut got_signal_yet = false;
    ctrlc::set_handler(move || {
        if !got_signal_yet {
            println!("[Ctrl+C]\nWorker #{} got stop signal", worker_id);
            got_signal_yet = true;
            return;
        }
        println!("Worker #{} say au revoir", worker_id);
        process::exit(0);
    })
    .expect("failed while setting Ctrl-C handler");
}

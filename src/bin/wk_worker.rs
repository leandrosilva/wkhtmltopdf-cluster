use clap::{App, Arg};
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use wkhtmltopdf_cluster::helpers::fs_helpers::create_dir_if_not_exists;
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
                )
                .arg(
                    Arg::with_name("timeout")
                        .about("max seconds per request")
                        .short('t')
                        .long("timeout")
                        .takes_value(true)
                        .value_name("TIMEOUT")
                        .default_value("5"),
                ),
        );

    let stop_signal = Arc::new(AtomicBool::new(false));
    watch_stop_signal(stop_signal.clone());

    let matches = app.get_matches_mut();
    match matches.subcommand() {
        ("start", Some(sub_matches)) => {
            let output_dir = Path::new(sub_matches.value_of("output").unwrap());
            create_dir_if_not_exists(&output_dir).expect("failed to create directory");

            let timeout = Duration::from_secs(
                sub_matches
                    .value_of("timeout")
                    .unwrap()
                    .parse::<u64>()
                    .expect("failed to parse timeout argument"),
            );

            let worker_id = process::id();

            println!("WkHTMLtoPDF Cluster :: Worker :: Start [#{}]", worker_id);
            let mut worker = Worker::new(worker_id, stop_signal.clone(), output_dir, timeout);
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

fn watch_stop_signal(stop_signal: Arc<AtomicBool>) {
    let worker_id = process::id();
    ctrlc::set_handler(move || {
        if !stop_signal.load(Ordering::SeqCst) {
            println!("[#{}] Worker got stop signal (Ctrl+C)", worker_id);
            stop_signal.store(true, Ordering::SeqCst);

            thread::spawn(move || {
                // TODO: get this tolerance from user on start up
                println!("[#{}] Will await for 5 secs max...", worker_id);
                thread::sleep(Duration::from_secs(5));
                println!("[#{}] Forced quit!", worker_id);
                process::exit(666);
            });

            return;
        }
        println!("Worker #{} say au revoir", worker_id);
        process::exit(0);
    })
    .expect("failed while setting Ctrl-C handler");
}

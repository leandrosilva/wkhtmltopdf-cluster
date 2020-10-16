use clap::{App, Arg};
use ctrlc;
use std::env;
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use wkhtmltopdf_cluster::broker::Broker;

// $ cargo run -p wkhtmltopdf-cluster --bin broker start -i 2
//

fn main() {
    let mut app = App::new("WkHTMLtoPDF Cluster")
        .version("1.0")
        .author("Leandro Silva <leandrodoze@gmail.com>")
        .about("This is the worker nodes manager.")
        .subcommand(
            App::new("start")
                .about("Starts the cluster manager for a number of worker nodes")
                .arg(
                    Arg::with_name("instances")
                        .about("number of workers")
                        .short('i')
                        .long("instances")
                        .takes_value(true)
                        .value_name("NUMBER")
                        .required(true),
                )
                .arg(
                    Arg::with_name("worker")
                        .about("worker node's binary path")
                        .short('w')
                        .long("worker")
                        .takes_value(true)
                        .value_name("PATH")
                        .required(false),
                )
                .arg(
                    Arg::with_name("output")
                        .about("output directory")
                        .short('o')
                        .long("output")
                        .takes_value(true)
                        .value_name("DIR")
                        .required(false),
                ),
        );

    let stop_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let matches = app.get_matches_mut();
    match matches.subcommand() {
        ("start", Some(sub_matches)) => {
            let w_instances: usize = sub_matches.value_of_t("instances").unwrap();
            let w_binpath: String = sub_matches
                .value_of_t("worker")
                .unwrap_or_else(|_err| get_default_worker_path());
            let w_output: String = sub_matches
                .value_of_t("output")
                .unwrap_or_else(|_err| get_default_output_dir());

            println!("WkHTMLtoPDF Cluster :: Manager :: Start");
            let broker = Arc::new(RwLock::new(Broker::new(
                w_instances,
                Path::new(&w_binpath),
                Path::new(&w_output),
            )));
            watch_stop_signal(stop_signal.clone(), broker.clone());
            broker
                .clone()
                .write()
                .expect("failed to acquire lock on broker to start it")
                .start(|pids| {
                    println!("All workers are up & running:");
                    for pid in pids {
                        println!("- Worker PID: {}", pid);
                    }
                })
                .expect("failed to start broker");
            println!("WkHTMLtoPDF Cluster :: Manager :: End");
        }
        ("", None) => app.print_help().unwrap(),
        _ => unreachable!(),
    }
}

fn watch_stop_signal(stop_signal: Arc<AtomicBool>, broker: Arc<RwLock<Broker>>) {
    let stop = stop_signal.clone();
    ctrlc::set_handler(move || {
        println!("Received Ctrl+C\nShutting down...");
        stop.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let stop = stop_signal.clone();
    let broker = broker.clone();
    std::thread::spawn(move || loop {
        if stop.load(Ordering::SeqCst) {
            Broker::send_stop_signal().expect("failed to send stop signal to broker");
            broker
                .read()
                .expect("failed to acquire lock on broker to stop it")
                .stop()
                .expect("failed to stop broker");
            println!("Bye, bye!");
            process::exit(0);
        }
        thread::sleep(Duration::from_secs(1));
    });
}

fn get_default_worker_path() -> String {
    let mut current_dir = env::current_exe().unwrap();
    current_dir.pop();
    current_dir.push("wk_worker");
    String::from(current_dir.to_str().unwrap())
}

fn get_default_output_dir() -> String {
    String::from("./examples/pdf")
}

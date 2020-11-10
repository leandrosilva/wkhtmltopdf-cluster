use clap::{App, Arg};
use ctrlc;
use std::env;
use std::path::Path;
use std::process;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
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
            let w_instances: usize = sub_matches.value_of_t("instances").unwrap();
            let w_binpath: String = sub_matches
                .value_of_t("worker")
                .unwrap_or_else(|_err| get_default_worker_path());
            let w_output: String = sub_matches
                .value_of_t("output")
                .unwrap_or_else(|_err| get_default_output_dir());
            let w_timeout = Duration::from_secs(
                sub_matches
                    .value_of("timeout")
                    .unwrap()
                    .parse::<u64>()
                    .expect("failed to parse timeout argument"),
            );
    
            let broker_id = process::id();

            println!("WkHTMLtoPDF Cluster :: Manager :: Start [#{}]", broker_id);
            let broker = Arc::new(RwLock::new(Broker::new(
                broker_id,
                stop_signal.clone(),
                w_instances,
                Path::new(&w_binpath),
                Path::new(&w_output),
                w_timeout
            )));
            broker
                .clone()
                .write()
                .expect("failed to acquire lock on broker to start it")
                .run(|worker_pids| {
                    println!("All workers are up & running:");
                    for worker_pid in worker_pids {
                        println!("- Worker PID: {}", worker_pid);
                    }
                })
                .expect("failed on running broker");
            println!("WkHTMLtoPDF Cluster :: Manager :: End [#{}]", broker_id);
            println!("Bye, bye!");
            process::exit(0);
        }
        ("", None) => app.print_help().unwrap(),
        _ => unreachable!(),
    }
}

fn watch_stop_signal(stop_signal: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        if !stop_signal.load(Ordering::SeqCst) {
            println!("[Ctrl+C]\nShutting down...");
            stop_signal.store(true, Ordering::SeqCst);
            Broker::send_stop_signal(5).expect("failed to send stop signal to broker");
            return;
        }
        println!("Bye, bye!");
        process::exit(0);
    })
    .expect("failed while setting Ctrl-C handler");
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

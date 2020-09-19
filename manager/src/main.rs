use clap::{App, Arg};
use error_chain::*;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

error_chain! {}

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
            match run_workers(w_instances, Path::new(&w_binpath), Path::new(&w_output)) {
                Ok(()) => println!("Work done"),
                Err(reason) => eprintln!("Failed due to: {}", reason),
            }
            println!("WkHTMLtoPDF Cluster :: Manager :: End");
        }
        ("", None) => app.print_help().unwrap(),
        _ => unreachable!(),
    }
}

fn get_default_worker_path() -> String {
    let mut current_dir = env::current_exe().unwrap();
    current_dir.pop();
    current_dir.push("wkhtmltopdf-cluster-worker");
    String::from(current_dir.to_str().unwrap())
}

fn get_default_output_dir() -> String {
    String::from("./examples/pdf")
}

fn run_workers(w_instances: usize, w_binpath: &Path, w_output: &Path) -> Result<()> {
    for i in 0..w_instances {
        println!(">> Worker #{}", i);

        let output = Command::new(w_binpath)
            .arg("start")
            .arg("--output")
            .arg(w_output.to_str().unwrap())
            .output()
            .expect(format!("Failed to start #{} {:?}", i, w_binpath).as_str());

        println!("Exit code: {}", output.status.code().unwrap_or(666));
        if output.status.success() {
            io::stdout().write_all(&output.stdout).unwrap();
        } else {
            error_chain::bail!(format!("Command #{} executed with failing error code", i).as_str());
        }
    }

    Ok(())
}

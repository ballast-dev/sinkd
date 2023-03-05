extern crate clap;
extern crate crossbeam;
extern crate notify;
extern crate regex;
extern crate serde;
extern crate toml;
#[macro_use]
extern crate log;
extern crate libc;
extern crate rpassword;

mod client;
mod config;
mod outcome;
mod fancy;
mod ipc;
mod server;
mod shiplog;
mod sinkd;
mod test;
mod utils;

use clap::{Arg, ArgAction, Command};
use outcome::Outcome;
use std::path::Path;

pub fn build_sinkd() -> Command {
    Command::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(Command::new("add")
            .about("Adds PATH to watch list\nlets sinkd become 'aware' of file or folder location provided")
            .arg(Arg::new("share")
                .short('s')
                .long("share")
                .value_name("SHARE")
                .num_args(1)
                .action(ArgAction::Append)
                .help("add watch for multiple users")
            )
            .arg(Arg::new("path")
                .num_args(0..)
                .help("sinkd starts watching path")
            )
            .override_usage("sinkd add FILE [FILE..]")
        )
        .subcommand(Command::new("ls")
            .alias("list")
            .about("List currently watched files from given PATH")
            .arg(Arg::new("PATHS")
                // need to revisit, should user have explicit control
                // possible -r flag for recursive 
                .required(false)
                .num_args(0..)
                .help("list watched files and directories")
            )
            .override_usage("sinkd ls [PATH..]")
        )
        .subcommand(Command::new("rm")
            .alias("remove")
            .about("Removes PATH from list of watched directories")
            .arg(Arg::new("PATH")
                .required(true)
                .num_args(1..)
            )
            .override_usage("sinkd rm PATH")
        )
        .subcommand(Command::new("start")
            .about("Starts the daemon")
            .override_usage("sinkd start [--client | --server]")
            .arg(Arg::new("CLIENT")
                .short('c')
                .long("client")
                .action(ArgAction::SetTrue)
                .help("start sinkd in client mode")
            )
            .arg(Arg::new("SERVER")
                .short('s')
                .long("server")
                .conflicts_with("CLIENT")
                .action(ArgAction::SetTrue)
                .help("start sinkd in server mode")
            )
            .arg(Arg::new("clear-logs")
                .long("clear-logs")
                .hide(true)
                .action(ArgAction::SetTrue)
            )
        )
        .subcommand(Command::new("stop")
            .about("Stops daemon")
        )
        .subcommand(Command::new("restart")
            .about("Restarts sinkd, reloading configuration")
        )
        .subcommand(Command::new("log")
            .about("test out logging")
        )
        .arg(Arg::new("verbose")
            .short('v')
            .action(ArgAction::Count)
            .help("verbose output")
        )
}

// user notification of operation
fn handle_outcome<T>(outcome: Outcome<T>) {
    match outcome {
        Ok(_) => { println!("operation completed successfully") },
        Err(e) => { println!("{:?}", e) }
    }
}

#[allow(dead_code)]
fn main() {
    println!("Running sinkd at {}", utils::get_timestamp("%T"));

    let matches = build_sinkd().get_matches();
    let verbosity = matches.get_count("verbose");

    if verbosity > 0 {
        println!("verbosity!: {}", verbosity);
    }

    match matches.subcommand() {
        Some(("add", submatches)) => {
            let mut share_paths = Vec::<&String>::new();
            let mut user_paths = Vec::<&String>::new();

            match submatches.get_many::<String>("share") {
                Some(shares) => {
                    share_paths = shares
                        .filter_map(|p| if Path::new(p).exists() { Some(p) } else { None })
                        .collect();
                }
                None => (),
            }

            match submatches.get_many::<String>("path") {
                Some(paths) => {
                    user_paths = paths
                        .filter_map(|p| if Path::new(p).exists() { Some(p) } else { None })
                        .collect();
                }
                None => (),
            }

            for p in &share_paths {
                println!("share.... {}", p);
            }
            for p in &user_paths {
                println!("regular... {}", p);
            }
        }
        Some(("ls", submatches)) => {
            if !submatches.args_present() {
                sinkd::list(None);
            } else {
                let vals: Vec<&str> = submatches
                    .get_many::<String>("PATHS")
                    .unwrap()
                    .map(|s| s.as_str())
                    .collect();
                sinkd::list(Some(&vals))
            }
        }
        Some(("rm", _)) => {
            sinkd::remove();
        }
        Some(("start", submatches)) => {
            let clear_logs = *submatches.get_one::<bool>("clear-logs").unwrap_or(&false);
            if submatches.args_present() {
                println!("Logging to: '{}'", utils::LOG_PATH);
                if submatches.get_flag("SERVER") {
                    if let Err(e) = server::start(verbosity, clear_logs) {
                        eprintln!("{}", e);
                    }
                } else if submatches.get_flag("CLIENT") {
                    if let Err(e) = client::start(verbosity, clear_logs) {
                        eprintln!("{}", e);
                    }
                }
            } else {
                eprintln!("Need know which to start --server or --client?")
            }
        }
        Some(("stop", _)) => {
            sinkd::stop();
        }
        Some(("restart", _)) => sinkd::restart(),
        Some(("log", _)) => sinkd::log(),
        _ => {
            println!("TODO: print help");
        }
    }
}

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
mod defs;
mod fancy;
mod ipc;
mod server;
mod shiplog;
mod sinkd;
mod test;
mod utils;

use clap::{Arg, ArgAction, Command};

pub fn build_sinkd() -> Command {
    Command::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
// let cfg = Arg::new("config")
//       .short('c')
//       .long("config")
//       .action(ArgAction::Set)
//       .value_name("FILE")
//       .help("Provides a config file to myprog");
        .subcommand(Command::new("add")
            .about("Adds PATH to watch list\nlets sinkd become 'aware' of file or folder location provided")
            .arg(Arg::new("share")
                .short('s')
                .long("share")
                .value_name("SHARE")
                .help("add watch for multiple users")
            )
            .arg(Arg::new("PATH")
                .required(true)
                .num_args(1..)
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
                .num_args(0)
                .conflicts_with("CLIENT")
                .help("start sinkd in server mode")
            )
            .arg(Arg::new("clear-logs")
                .long("clear-logs")
                .hide(true)
                .action(clap::ArgAction::SetTrue)
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
        // .arg(Arg::new("reconfigure")
        //     .long("reconfigure")
        //     .help("verbose output")
        // )
}

#[allow(dead_code)]
fn main() {
    println!("Running sinkd at {}", utils::get_timestamp("%T"));

    let matches = build_sinkd().get_matches();
    let verbosity = matches.get_count("verbose");
    println!("verbosity!: {}", verbosity);
    
    // match matches.occurrences_of("reconfigure") {
    //     0 => (),
    //     _ => {
    //         // sinkd::reparse();
    //         println!("Reloaded configuration");
    //         return;
    //     }
    // }

    match matches.subcommand() {
        // Some(("add", submatches)) => {
        //     for path in submatches.values_of("PATH").unwrap() {
        //         if std::path::Path::new(path).exists() {
        //             sinkd::add(path);
        //         } else {
        //             println!("'{}' does not exist", path);
        //         }
        //     }
        // }
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
                // if submatches.is_present("SERVER") {
                //     if let Err(e) = server::start(verbosity, clear_logs) {
                //         eprintln!("{}", e);
                //     }
                // } else if submatches.is_present("CLIENT") {
                //     if let Err(e) = client::start(verbosity, clear_logs) {
                //         eprintln!("{}", e);
                //     }
                // }
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

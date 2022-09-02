extern crate clap;
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
mod daemon;
mod fancy;
mod ipc;
mod server;
mod shiplog;
mod sinkd;
mod test;
mod utils;

use clap::{App, Arg, Command};

pub fn build_sinkd() -> App<'static> {
    App::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(Command::new("add")
            .about("Adds PATH to watch list\nlets sinkd become 'aware' of file or folder location provided")
            .arg(Arg::with_name("SHARE")
                .short('s')
                .long("share")
                .help("add watch for multiple users")
            )
            .arg(Arg::new("PATH")
                .required(true)
                .multiple_occurrences(true) // CAREFUL: this will consume other arguments
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
                .multiple_occurrences(true) // CAREFUL: this will consume other arguments
                .help("list watched files and directories")
            )
            .override_usage("sinkd ls [PATH..]")
        )
        .subcommand(Command::new("rm")
            .alias("remove")
            .about("Removes PATH from list of watched directories")
            .arg(Arg::new("PATH")
                .required(true)
                .multiple_occurrences(true) // CAREFUL: this will consume other arguments
            )
            .override_usage("sinkd rm PATH")
        )
        .subcommand(Command::new("start")
            .about("Starts the daemon")
            .usage("sinkd start [--client | --server]")
            .arg(Arg::with_name("CLIENT")
                .short('c')
                .long("client")
                .takes_value(false)
                .help("start sinkd in client mode")
            )
            .arg(Arg::with_name("SERVER")
                .short('s')
                .long("server")
                .takes_value(false)
                .conflicts_with("CLIENT")
                .help("start sinkd in server mode")
            )
            .arg(Arg::with_name("clear-logs")
                .long("clear-logs")
                .hidden(true)
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
        .arg(Arg::with_name("verbose")
            .short('v')
            .multiple_occurrences(true)
            .help("verbose output")
        )
        .arg(Arg::with_name("reconfigure")
            .long("reconfigure")
            .help("verbose output")
        )
}

#[allow(dead_code)]
fn main() {
    println!("Running sinkd at {}", utils::get_timestamp("%Y%m%d-%T"));

    // mqtt::listen();
    // std::process::exit(0);
    let matches = build_sinkd().get_matches();
    let mut verbosity: u8 = 0;
    match matches.occurrences_of("verbose") {
        1 => verbosity = 1,
        2 => verbosity = 2,
        3 => verbosity = 3,
        _ => (),
    }

    match matches.occurrences_of("reconfigure") {
        0 => (),
        _ => {
            // sinkd::reparse();
            println!("Reloaded configuration");
            return;
        }
    }

    match matches.subcommand() {
        Some(("add", submatches)) => {
            for path in submatches.values_of("PATH").unwrap() {
                if std::path::Path::new(path).exists() {
                    sinkd::add(path);
                } else {
                    println!("'{}' does not exist", path);
                }
            }
        }
        Some(("ls", submatches)) => {
            if !submatches.args_present() {
                sinkd::list(None);
            } else {
                let vals: Vec<&str> = submatches.get_many::<String>("PATHS")
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
            if submatches.is_present("SERVER") {
                server::start(verbosity, clear_logs);
            } else {
                if let Err(error) = client::start(verbosity, clear_logs) {
                    eprintln!("{}", error);
                    println!("unable to start client, take a look: {}", utils::LOG_PATH)
                }
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

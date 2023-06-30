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

#[macro_use]
mod outcome;
mod client;
mod config;
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

use crate::utils::Parameters;

#[rustfmt::skip]
pub fn build_sinkd() -> Command {
    Command::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::new("sys-cfg-file")
            .short('s')
            .long("system-config")
            .num_args(1)
            .default_value("/etc/sinkd.conf")
            .help("system configuration file to use")
        )
        .arg(Arg::new("usr-cfg-file")
            .short('u')
            .long("user-config")
            .num_args(1)
            .default_value("~/.config/sinkd.conf")
            .help("user configuration file to use")
        )
        .subcommand(Command::new("add")
            .about("Adds PATH to watch list")
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
            .about("Removes PATH from watch list")
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
        .arg(Arg::new("debug")
            .short('d')
            .long("debug")
            .action(ArgAction::SetTrue)
            .help("debug mode: log files to /tmp")
        )
}

fn handle_outcome<T>(outcome: Outcome<T>) {
    match outcome {
        Ok(_) => println!("operation completed successfully"),
        Err(e) => eprintln!("ERROR: {}", e)
    }
}

#[allow(dead_code)]
fn main() {
    println!("Running sinkd at {}", utils::get_timestamp("%T"));

    let mut cli = build_sinkd();
    let matches = cli.get_matches_mut();
    // let verbosity = matches.get_count("verbose");
    // let clear_logs = *submatches.get_one::<bool>("clear-logs").unwrap_or(&false);
    let params: Parameters;
    if matches.get_flag("debug") {
        params = Parameters::debug();
    } else {
        params = Parameters::new();
    }

    let sys_cfg = match utils::resolve(matches.get_one::<String>("sys-cfg-file").unwrap()) {
        Ok(normalized) => normalized,
        Err(e) => return eprintln!("system config path error: {}", e)
    };

    let usr_cfg = match utils::resolve(matches.get_one::<String>("usr-cfg-file").unwrap()) {
        Ok(normalized) => normalized,
        Err(e) => return eprintln!("user config path error: {}", e)
    };

    println!("{}", sys_cfg.display());
    println!("{}", usr_cfg.display());

    // if verbosity > 0 {
    //     println!("verbosity!: {}", verbosity);
    // }

    match matches.subcommand() {
        Some(("add", submatches)) => {
            let mut share_paths = Vec::<&String>::new();
            let mut user_paths = Vec::<&String>::new();

            if let Some(shares) = submatches.get_many::<String>("share") {
                share_paths = shares.filter(|p| Path::new(p).exists()).collect();
            }

            if let Some(paths) = submatches.get_many::<String>("path") {
                user_paths = paths.filter(|p| Path::new(p).exists()).collect();
            }

            sinkd::add(share_paths, user_paths);
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
            // TODO: check to see if broker is up!!!
            if submatches.get_flag("SERVER") {
                handle_outcome(server::start(&params));
            } else if submatches.get_flag("CLIENT") {
                handle_outcome(client::start(&params));
            } else {
                eprintln!("Need know which to start --server or --client?")
            }
        }
        Some(("stop", _)) => {
            // sinkd::stop();
        }
        Some(("restart", _)) => sinkd::restart(),
        Some(("log", _)) => sinkd::log(&params),
        _ => {
            cli.print_help().expect("sinkd usage: .... ");
        }
    }
}

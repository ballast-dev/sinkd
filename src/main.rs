extern crate clap;
extern crate crossbeam;
extern crate notify;
extern crate regex;
extern crate serde;
extern crate toml;
#[macro_use(debug, info, warn, error)]
extern crate log;
extern crate libc;
extern crate nix;
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
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use crate::utils::Parameters;

#[rustfmt::skip]
pub fn build_sinkd() -> Command {
    Command::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
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
                .value_name("PATH")
                .num_args(0..)
                .help("sinkd starts watching path")
            )
        )
        .subcommand(Command::new("ls")
            .alias("list")
            .about("List currently watched files from given PATH")
            .arg(Arg::new("path")
                .value_name("PATH")
                // need to revisit, should user have explicit control
                // possible -r flag for recursive 
                .required(false)
                .num_args(0..)
                .help("list watched files and directories")
            )
        )
        .subcommand(Command::new("rm")
            .alias("remove")
            .about("Removes PATH from watch list")
            .arg(Arg::new("path")
                .value_name("PATH")
                .required(true)
                .num_args(1..)
            )
        )
        .subcommand(Command::new("start")
            .about("Starts the daemon")
            // .override_usage("sinkd start [--client | --server]")
            .arg(Arg::new("client")
                .value_name("CLIENT")
                .short('c')
                .long("client")
                .action(ArgAction::SetTrue)
                .conflicts_with("server")
                .help("start sinkd in client mode")
            )
            .arg(Arg::new("server")
                .value_name("SERVER")
                .short('s')
                .long("server")
                .conflicts_with("client")
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
            .global(true)
        )
        .arg(Arg::new("system-config")
            .long("sys-cfg")
            .num_args(1)
            .default_value("/etc/sinkd.conf")
            .help("system configuration file to use")
            .global(true)
        )
        .arg(Arg::new("user-configs")
            .long("usr-cfg")
            .num_args(1)
            .action(ArgAction::Append)
            .default_value("~/.config/sinkd.conf")
            //? long help is '--help' versus '-h'
            .long_help("providing this flag will override supplied users in system config")
            .help("user configuration files to use")
            .global(true)
        )
}

fn egress<T>(outcome: Outcome<T>) -> ExitCode {
    match outcome {
        Ok(_) => {
            fancy::println("operation completed successfully", 
            fancy::Attrs::NORMAL, fancy::Colors::GREEN);
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            error!("{}", e);
            fancy_error!("ERROR: {}", e);
            std::process::ExitCode::FAILURE
        }
    }
}

#[allow(dead_code)]
fn main() -> ExitCode {
    println!("Running sinkd at {}", utils::get_timestamp("%T"));

    let mut cli = build_sinkd();
    let matches = cli.get_matches_mut();

    let system_cfg = match utils::resolve(matches.get_one::<String>("system-config").unwrap()) {
        Ok(normalized) => {
            if normalized.is_dir() {
                // TODO: have error codes
                return egress::<String>(bad!(
                    "{} is a directory not a file, aborting",
                    normalized.display()
                ));
            } else {
                normalized
            }
        }
        Err(e) => return egress::<String>(bad!("system config path error: {}", e)),
    };

    let user_cfgs = match matches.get_many::<String>("user-configs") {
        Some(passed_configs) => {
            let mut user_configs: Vec<PathBuf> = vec![];
            for passed_config in passed_configs {
                let _path = match utils::resolve(passed_config) {
                    Ok(normalized) => {
                        if normalized.is_dir() {
                            return egress::<String>(bad!(
                                "{} is a directory not a file, aborting",
                                normalized.display()
                            ));
                        } else {
                            normalized
                        }
                    }
                    Err(e) => return egress::<String>(bad!("config path error: {}", e)),
                };
                user_configs.push(_path);
            }
            Some(user_configs)
        }
        None => None,
    };

    let params = Parameters::new(
        matches.get_count("verbose"),
        matches.get_flag("debug"),
        &system_cfg,
        &user_cfgs,
    );

    if params.verbosity >= 3 {
        fancy_debug!("system config: {}", &system_cfg.display());
        for user_cfg in &user_cfgs.unwrap() {
            fancy_debug!("user config: {}", user_cfg.display());
        }
    }

    match matches.subcommand() {
        Some(("add", submatches)) => {
            let mut share_paths = Vec::<&String>::new();
            let mut user_paths = Vec::<&String>::new();

            // TODO: combine this logic into function
            if let Some(shares) = submatches.get_many::<String>("share") {
                share_paths = shares.filter(|p| Path::new(p).exists()).collect();
            }

            // TODO: move into add call
            if let Some(paths) = submatches.get_many::<String>("path") {
                user_paths = paths
                    .filter(|p| {
                        let p = Path::new(p);
                        if p.exists() {
                            true
                        } else {
                            fancy::println(
                                &format!("path doesn't exist: {}", &p.display()),
                                fancy::Attrs::BOLD,
                                fancy::Colors::RED,
                            );
                            false
                        }
                    })
                    .collect();
            }

            egress(sinkd::add(share_paths, user_paths))
        }
        Some(("ls", submatches)) => {
            // only list out tracking folders and files
            if let Some(paths) = submatches.get_many::<String>("path") {
                let tracked_paths: Vec<&String> = paths
                    .filter(|p| {
                        let p = Path::new(p);
                        if p.exists() { // TODO: check against loaded config
                            true
                        } else {
                            fancy::println(
                                &format!("path doesn't exist: {}", &p.display()),
                                fancy::Attrs::BOLD,
                                fancy::Colors::RED,
                            );
                            false
                        }
                    }).collect();
                egress(sinkd::list(Some(tracked_paths)))
            } else {
                egress(sinkd::list(None))
            }
        }
        Some(("rm", _)) => egress(sinkd::remove()),
        Some(("start", submatches)) => {
            // TODO: check to see if broker is up!!!
            if submatches.get_flag("server") {
                egress(server::start(&params))
            } else if submatches.get_flag("client") {
                egress(client::start(&params))
            } else {
                egress::<String>(bad!("Need know which to start --server or --client?"))
            }
        }
        Some(("stop", _)) => {
            egress::<String>(bad!("under maintenance"))
            // sinkd::stop();
        }
        Some(("restart", _)) => {
            egress::<String>(bad!("under maintenance"))
            // sinkd::restart(),
        }
        Some(("log", _)) => egress(sinkd::log(&params)),
        _ => {
            cli.print_help().expect("sinkd usage: .... ");
            ExitCode::from(ExitCode::SUCCESS)
        }
    }
}

extern crate clap;
extern crate crossbeam;
extern crate notify;
extern crate serde;
extern crate toml;
#[macro_use(debug, info, warn, error)]
extern crate log;
extern crate libc;
extern crate nix;

#[macro_use]
mod cli;
mod client;
mod config;
mod fancy;
mod flags;
mod ipc;
#[macro_use]
mod outcome;
mod parameters;
mod server;
mod shiplog;
mod sinkd;
mod test;
mod time;

use clap::parser::ValuesRef;
use outcome::Outcome;
use std::{io::Write, path::Path, process::ExitCode};

use crate::parameters::{DaemonType, Parameters};

fn check_path(p: &str) -> bool {
    let p = Path::new(p);
    if p.exists() {
        true
    } else {
        fancy::println(
            &format!("path doesn't exist: {}", &p.display()),
            fancy::Attrs::Bold,
            fancy::Colors::Red,
        );
        false
    }
}

fn egress<T>(outcome: Outcome<T>) -> ExitCode {
    match outcome {
        Ok(_) => {
            //fancy::println(
            //    "operation completed successfully",
            //    fancy::Attrs::Normal,
            //    fancy::Colors::Green,
            //);
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            error!("{}", e);
            fancy_error!("ERROR: {}", e);
            std::process::ExitCode::FAILURE
        }
    }
}

// fn args_to_command_line() -> String {
//     let args: Vec<String> = std::env::args().skip(1).collect(); // Skip the first argument (exe path)
//     args.iter()
//         .map(|arg| {
//             // Escape arguments with quotes if they contain spaces or special characters
//             if arg.contains(' ') || arg.contains('"') || arg.contains('\\') {
//                 let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
//                 format!("\"{}\"", escaped)
//             } else {
//                 arg.clone()
//             }
//         })
//         .collect::<Vec<String>>()
//         .join(" ") // Join all args with spaces
// }

// FIXME:
// TODO:
// NOTE:
// HACK:
// WARNING:

#[allow(dead_code)]
fn main() -> ExitCode {
    println!("timestamp {}", time::stamp(None));

    let mut cli = crate::cli::build_sinkd();
    let matches = cli.get_matches_mut();

    // println!("{:?}", matches);
    let (system_config, user_configs, daemon_type) = match matches.subcommand() {
        // NOTE: a bit of a mismatch as first run will always be UnixClient
        
        Some(("client", submatches)) => {
            let system_config = submatches.get_one("system-config");
            let user_configs = submatches.get_many("user-configs");
            let daemon_type = if matches.get_flag("windows-daemon") {
                // let mut f = std::fs::File::create("windows_daemon.txt").unwrap();
                // f.write(b"yay!?")?;
                DaemonType::WindowsClient
            } else {
                DaemonType::UnixClient
            };
            (system_config, user_configs, daemon_type)
        }
        _ => {
            let daemon_type = if matches.get_flag("windows-daemon") {
                DaemonType::WindowsServer
            } else {
                DaemonType::UnixServer
            };
            (None, None, daemon_type)
        }
    };

    let params = match Parameters::from(
        daemon_type,
        matches.get_count("verbose"),
        matches.get_count("debug"),
        system_config,
        user_configs,
    ) {
        Ok(params) => params,
        Err(e) => return egress::<String>(bad!(e)),
    };

    if params.debug > 0 {
        println!("{}", &params);
    }

    match matches.subcommand() {
        Some(("server", submatches)) => match submatches.subcommand() {
            Some(("start", _)) => egress(server::start(&params)),
            Some(("restart", _)) => egress(server::restart(&params)),
            Some(("stop", _)) => egress(server::stop()),
            _ => {
                cli.print_help().expect("sinkd usage: .... ");
                ExitCode::SUCCESS
            }
        },
        Some(("client", submatches)) => match submatches.subcommand() {
            Some(("start", _)) => egress(client::start(&params)),
            Some(("restart", _)) => egress(client::restart(&params)),
            Some(("stop", _)) => egress(client::stop(&params)),
            _ => {
                cli.print_help().expect("sinkd usage: .... ");
                ExitCode::SUCCESS
            }
        },
        Some(("add", submatches)) => {
            let mut share_paths = Vec::<&String>::new();
            let mut user_paths = Vec::<&String>::new();

            if let Some(shares) = submatches.get_many::<String>("share") {
                share_paths = shares.filter(|p| check_path(p)).collect();
            }
            if let Some(paths) = submatches.get_many::<String>("path") {
                user_paths = paths.filter(|p| check_path(p)).collect();
            }

            egress(sinkd::add(share_paths, user_paths))
        }
        Some(("rm", submatches)) => {
            let mut share_paths = Vec::<&String>::new();
            let mut user_paths = Vec::<&String>::new();

            if let Some(shares) = submatches.get_many::<String>("share") {
                share_paths = shares.filter(|p| check_path(p)).collect();
            }
            if let Some(paths) = submatches.get_many::<String>("path") {
                user_paths = paths.filter(|p| check_path(p)).collect();
            }
            egress(sinkd::remove(share_paths, user_paths))
        }
        Some(("adduser", submatches)) => {
            let users = submatches.get_many::<String>("user");
            egress(sinkd::adduser(users))
        }
        Some(("rmuser", submatches)) => {
            let users = submatches.get_many::<String>("user");
            egress(sinkd::rmuser(users))
        }
        Some(("ls", submatches)) => {
            // only list out tracking folders and files
            if let Some(paths) = submatches.get_many::<String>("path") {
                let _paths = paths.filter(|p| check_path(p)).collect();
                egress(sinkd::list(Some(_paths)))
            } else {
                egress(sinkd::list(None))
            }
        }
        Some(("log", _)) => egress(sinkd::log(&params)),
        _ => {
            cli.print_help().expect("sinkd usage: .... ");
            ExitCode::SUCCESS
        }
    }
}

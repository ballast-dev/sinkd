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
mod ipc;
mod rsync;
#[macro_use]
mod outcome;
mod ops;
mod parameters;
mod server;
mod shiplog;
mod time;

use outcome::Outcome;
use std::{path::Path, process::ExitCode};

use crate::parameters::Parameters;

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

fn windoze() -> ExitCode {
    let cli = crate::cli::build_sinkd().no_binary_name(true);
    let matches = cli.get_matches();
    let params = Parameters::from(&matches).unwrap();
    if let Err(e) = shiplog::init(&params) {
        let _ = std::fs::write("sinkd_error.log", format!("{e:?}"));
    }
    info!("-- windoze --");
    match matches.subcommand() {
        Some(("client", _submatches)) => debug!("windows client!"),
        Some(("server", _submatches)) => debug!("windows server!"),
        _ => debug!("uh oh... matches>> {:?}", matches),
    }
    ExitCode::SUCCESS
}

// FIXME:
// TODO:
// NOTE:
// HACK:
// WARNING:

#[allow(dead_code)]
fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--windows-daemon") {
        return windoze();
    }

    println!("timestamp {}", time::stamp(None));

    let mut cli = crate::cli::build_sinkd();
    let matches = cli.get_matches_mut();

    let params = match Parameters::from(&matches) {
        Ok(params) => params,
        Err(e) => return egress::<String>(bad!(e)),
    };

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

            egress(ops::add(share_paths, user_paths))
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
            egress(ops::remove(share_paths, user_paths))
        }
        Some(("adduser", submatches)) => {
            let users = submatches.get_many::<String>("user");
            egress(ops::adduser(users))
        }
        Some(("rmuser", submatches)) => {
            let users = submatches.get_many::<String>("user");
            egress(ops::rmuser(users))
        }
        Some(("ls", submatches)) => {
            // only list out tracking folders and files
            if let Some(paths) = submatches.get_many::<String>("path") {
                let _paths = paths.filter(|p| check_path(p)).collect();
                egress(ops::list(Some(_paths)))
            } else {
                egress(ops::list(None))
            }
        }
        Some(("log", _)) => egress(ops::log(&params)),
        _ => {
            cli.print_help().expect("sinkd usage: .... ");
            ExitCode::SUCCESS
        }
    }
}

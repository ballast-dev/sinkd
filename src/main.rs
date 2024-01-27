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
mod cli;
mod client;
mod config;
mod fancy;
mod ipc;
mod server;
mod shiplog;
mod sinkd;
mod test;
mod utils;

use outcome::Outcome;
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use crate::utils::Parameters;

fn check_path(p: &str) -> bool {
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
}

fn egress<T>(outcome: Outcome<T>) -> ExitCode {
    match outcome {
        Ok(_) => {
            fancy::println(
                "operation completed successfully",
                fancy::Attrs::NORMAL,
                fancy::Colors::GREEN,
            );
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

    let mut cli = crate::cli::build_sinkd();
    let matches = cli.get_matches_mut();

    // println!("{:?}", matches);
    let daemon_type = if let Some(("server", _)) = matches.subcommand() {
        utils::DaemonType::Server
    } else {
        utils::DaemonType::Client 
    };

    let params = match Parameters::new(
        &daemon_type,
        matches.get_count("verbose"),
        matches.get_flag("debug"),
        // they both have a default value, so unwrap is safe here
        matches.get_one::<String>("system-config").unwrap(),
        matches.get_many::<String>("user-configs").unwrap(),
    ) {
        Ok(params) => params,
        Err(e) => return egress::<String>(bad!(e)),
    };

    if params.verbosity >= 3 {
        fancy_debug!("system config: {}", params.system_config.display());
        for user_cfg in params.user_configs.iter() {
            fancy_debug!("user config: {}", user_cfg.display());
        }
    }

    match matches.subcommand() {
        Some(("server", submatches)) => match submatches.subcommand() {
            Some(("start", _)) => egress(server::start(&params)),
            Some(("restart", _)) => egress(server::restart(&params)),
            Some(("stop", _)) => egress(server::stop(&params)),
            _ => {
                cli.print_help().expect("sinkd usage: .... ");
                ExitCode::from(ExitCode::SUCCESS)
            }
        },
        Some(("client", submatches)) => match submatches.subcommand() {
            Some(("start", _)) => egress(server::start(&params)),
            Some(("restart", _)) => egress(server::restart(&params)),
            Some(("stop", _)) => egress(server::stop(&params)),
            _ => {
                cli.print_help().expect("sinkd usage: .... ");
                ExitCode::from(ExitCode::SUCCESS)
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
            ExitCode::from(ExitCode::SUCCESS)
        }
    }
}

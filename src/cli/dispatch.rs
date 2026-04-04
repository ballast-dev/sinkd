use clap::ArgMatches;
use log::{debug, error, info};
use std::{path::Path, process::ExitCode};

use crate::outcome::Outcome;
use crate::parameters::DaemonParameters;
use crate::{client, ops, server, shiplog, time};

use super::build::build_sinkd;

fn check_path_exists(p: &str) -> bool {
    let p = Path::new(p);
    if p.exists() {
        true
    } else {
        crate::fancy::println(
            &format!("path doesn't exist: {}", &p.display()),
            crate::fancy::Attrs::Bold,
            crate::fancy::Colors::Red,
        );
        false
    }
}

fn egress<T>(outcome: Outcome<T>) -> ExitCode {
    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            fancy_error!("ERROR: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn windoze() -> ExitCode {
    let cli = build_sinkd().no_binary_name(true);
    let matches = cli.get_matches();
    let params = DaemonParameters::from_matches(&matches).unwrap();
    if let Err(e) = shiplog::init(params.shared()) {
        let _ = std::fs::write("sinkd_error.log", format!("{e:?}"));
    }
    info!("-- windoze --");
    match matches.subcommand() {
        Some(("client", _submatches)) => debug!("windows client!"),
        Some(("server", _submatches)) => debug!("windows server!"),
        _ => debug!("uh oh... matches>> {matches:?}"),
    }
    ExitCode::SUCCESS
}

fn collect_share_user_paths(submatches: &ArgMatches) -> (Vec<&String>, Vec<&String>) {
    let share_paths = submatches
        .get_many::<String>("share")
        .map(|shares| shares.filter(|p| check_path_exists(p)).collect())
        .unwrap_or_default();
    let user_paths = submatches
        .get_many::<String>("path")
        .map(|paths| paths.filter(|p| check_path_exists(p)).collect())
        .unwrap_or_default();
    (share_paths, user_paths)
}

fn cmd_server(submatches: &ArgMatches, params: &DaemonParameters) -> ExitCode {
    let DaemonParameters::Server(server) = params else {
        let err: Outcome<()> = bad!(
            "internal error: expected server parameters for server subcommand"
        );
        return egress(err);
    };
    let mut cli = build_sinkd();
    match submatches.subcommand() {
        Some(("start", _)) => egress(server::start(server)),
        Some(("restart", _)) => egress(server::restart(server)),
        Some(("stop", _)) => egress(server::stop()),
        _ => {
            let _ = cli.print_help();
            ExitCode::SUCCESS
        }
    }
}

fn cmd_client(submatches: &ArgMatches, params: &DaemonParameters) -> ExitCode {
    let DaemonParameters::Client(client) = params else {
        let err: Outcome<()> = bad!(
            "internal error: expected client parameters for client subcommand"
        );
        return egress(err);
    };
    let mut cli = build_sinkd();
    match submatches.subcommand() {
        Some(("start", _)) => egress(client::start(client)),
        Some(("restart", _)) => egress(client::restart(client)),
        Some(("stop", _)) => egress(client::stop(client)),
        _ => {
            let _ = cli.print_help();
            ExitCode::SUCCESS
        }
    }
}

fn cmd_add(submatches: &ArgMatches, params: &DaemonParameters) -> ExitCode {
    let DaemonParameters::Client(client) = params else {
        let err: Outcome<()> = bad!("add requires client configuration context");
        return egress(err);
    };
    let (share_paths, user_paths) = collect_share_user_paths(submatches);
    egress(ops::add(client, &share_paths, &user_paths))
}

fn cmd_rm(submatches: &ArgMatches, params: &DaemonParameters) -> ExitCode {
    let DaemonParameters::Client(client) = params else {
        let err: Outcome<()> = bad!("remove requires client configuration context");
        return egress(err);
    };
    let (share_paths, user_paths) = collect_share_user_paths(submatches);
    egress(ops::remove(client, &share_paths, &user_paths))
}

fn cmd_adduser(submatches: &ArgMatches, params: &DaemonParameters) -> ExitCode {
    let DaemonParameters::Client(client) = params else {
        let err: Outcome<()> = bad!("adduser requires client configuration context");
        return egress(err);
    };
    let users = submatches.get_many::<String>("user");
    egress(ops::adduser(client, users))
}

fn cmd_rmuser(submatches: &ArgMatches, params: &DaemonParameters) -> ExitCode {
    let DaemonParameters::Client(client) = params else {
        let err: Outcome<()> = bad!("rmuser requires client configuration context");
        return egress(err);
    };
    let users = submatches.get_many::<String>("user");
    egress(ops::rmuser(client, users))
}

fn cmd_ls(submatches: &ArgMatches, params: &DaemonParameters) -> ExitCode {
    let DaemonParameters::Client(client) = params else {
        let err: Outcome<()> = bad!("list requires client configuration context");
        return egress(err);
    };
    let list_server = submatches.get_flag("server");
    if let Some(paths) = submatches.get_many::<String>("path") {
        let filtered_paths = paths.filter(|p| check_path_exists(p)).collect();
        egress(ops::list(client, Some(filtered_paths), list_server))
    } else {
        egress(ops::list(client, None, list_server))
    }
}

fn cmd_log(params: &DaemonParameters) -> ExitCode {
    let DaemonParameters::Client(client) = params else {
        let err: Outcome<()> = bad!("log requires client configuration context");
        return egress(err);
    };
    egress(ops::log(client))
}

fn cmd_root_help() -> ExitCode {
    let mut cli = build_sinkd();
    let _ = cli.print_help();
    ExitCode::SUCCESS
}

#[must_use]
pub fn dispatch_sinkd_matches(matches: &ArgMatches) -> ExitCode {
    let params = match DaemonParameters::from_matches(matches) {
        Ok(params) => params,
        Err(e) => return egress::<String>(bad!(e)),
    };

    match matches.subcommand() {
        Some(("server", sub)) => cmd_server(sub, &params),
        Some(("client", sub)) => cmd_client(sub, &params),
        Some(("add", sub)) => cmd_add(sub, &params),
        Some(("rm", sub)) => cmd_rm(sub, &params),
        Some(("adduser", sub)) => cmd_adduser(sub, &params),
        Some(("rmuser", sub)) => cmd_rmuser(sub, &params),
        Some(("ls", sub)) => cmd_ls(sub, &params),
        Some(("log", _)) => cmd_log(&params),
        _ => cmd_root_help(),
    }
}

/// Full `sinkd` CLI entry (default binary).
#[must_use]
pub fn run_sinkd() -> ExitCode {
    if std::env::args().any(|arg| arg == "--windows-daemon") {
        return windoze();
    }

    let cli = build_sinkd();
    let matches = match cli.try_get_matches() {
        Ok(m) => m,
        Err(e) => e.exit(),
    };

    println!("timestamp {}", time::stamp(None));

    dispatch_sinkd_matches(&matches)
}

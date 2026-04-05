use clap::ArgMatches;
use log::{debug, info};
use std::process::ExitCode;

use crate::parameters::DaemonParameters;
use crate::time;


use log::error;

use crate::outcome::Outcome;

use super::build::build_sinkd;

pub(super) fn egress<T>(outcome: Outcome<T>) -> ExitCode {
    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            fancy_error!("ERROR: {}", e);
            ExitCode::FAILURE
        }
    }
}

// pub(super) fn print_subcommand_help(name: &str) -> ExitCode {
//     let mut root = build_sinkd();
//     if let Some(cmd) = root.find_subcommand_mut(name) {
//         let _ = cmd.print_help();
//     }
//     ExitCode::SUCCESS
// }


fn windoze() -> ExitCode {
    let cli = build_sinkd().no_binary_name(true);
    let matches = cli.get_matches();
    let params = DaemonParameters::from_matches(&matches).unwrap();
    if let Err(e) = shiplog::init(params.shared()) {
        let _ = std::fs::write("sinkd_error.log", format!("{e:?}"));
    }
    info!("-- windoze --");
    match matches.subcommand() {
        Some(("client", _)) => debug!("windows client!"),
        Some(("server", _)) => debug!("windows server!"),
        _ => debug!("uh oh... matches>> {matches:?}"),
    }
    ExitCode::SUCCESS
}

#[must_use]
pub fn dispatch_sinkd_matches(matches: &ArgMatches) -> ExitCode {
    let params = match DaemonParameters::from_matches(matches) {
        Ok(p) => p,
        Err(e) => return egress::<String>(bad!(e)),
    };

    match (matches.subcommand(), &params) {
        (Some(("server", sub)), DaemonParameters::Server(s)) => server::dispatch(sub, s),
        (Some(("client", sub)), DaemonParameters::Client(c)) => client::dispatch(sub, c),
        _ => {
            let mut cli = build_sinkd();
            let _ = cli.print_help();
            ExitCode::SUCCESS
        }
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

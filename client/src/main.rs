mod client;
mod cli;
mod daemon;
mod init;
mod parameters;
mod sync_state;

use std::process::ExitCode;

use log::{debug, info};
use sinkd_core::{shiplog, time};

use crate::parameters::ClientParameters;

fn windoze() -> ExitCode {
    let cli_cmd = cli::build_command().no_binary_name(true);
    let matches = match cli_cmd.try_get_matches() {
        Ok(m) => m,
        Err(e) => e.exit(),
    };
    let params = match ClientParameters::from_matches(&matches) {
        Ok(p) => p,
        Err(e) => {
            sinkd_core::fancy_error!("ERROR: {}", e);
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = shiplog::init(&params.shared) {
        let _ = std::fs::write("sinkd_error.log", format!("{e:?}"));
    }
    info!("-- windows client daemon --");
    match matches.subcommand() {
        Some(("start" | "restart", _)) => cli::egress(client::init(&params)),
        Some((name, _)) => {
            debug!("windows daemon unexpected subcommand: {name}");
            ExitCode::SUCCESS
        }
        None => ExitCode::SUCCESS,
    }
}

fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--windows-daemon") {
        return windoze();
    }

    let cmd = cli::build_command();
    let matches = match cmd.try_get_matches() {
        Ok(m) => m,
        Err(e) => e.exit(),
    };

    let params = match ClientParameters::from_matches(&matches) {
        Ok(p) => p,
        Err(e) => {
            sinkd_core::fancy_error!("ERROR: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if params.shared.debug > 0 {
        println!("timestamp {}", time::stamp(None));
    }

    cli::dispatch(&matches, &params)
}

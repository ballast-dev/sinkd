use std::process::ExitCode;

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

pub(super) fn print_subcommand_help(name: &str) -> ExitCode {
    let mut root = build_sinkd();
    if let Some(cmd) = root.find_subcommand_mut(name) {
        let _ = cmd.print_help();
    }
    ExitCode::SUCCESS
}

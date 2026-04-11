use clap::{ArgMatches, Command};
use std::process::ExitCode;

use crate::parameters::ServerParameters;
use crate::server;

// use super::common::{egress, print_subcommand_help};
use super::egress;

pub(super) fn build_command() -> Command {
    Command::new("server")
        .about("Server side daemon")
        .visible_alias("s")
        .subcommand(Command::new("start").about("Start the server daemon"))
        .subcommand(Command::new("restart").about("Restart the server daemon"))
        .subcommand(Command::new("stop").about("Stop the server daemon"))
}

pub(super) fn dispatch(sub: &ArgMatches, server: &ServerParameters) -> ExitCode {
    match sub.subcommand() {
        Some(("start", _)) => egress(server::start(server)),
        Some(("restart", _)) => egress(server::restart(server)),
        Some(("stop", _)) => egress(server::stop()),
        _ => {
            fancy_error!("unknown subcommand");
            ExitCode::FAILURE
        }
    }
}

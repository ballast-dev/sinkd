use clap::{Arg, ArgAction, ArgMatches, Command};
use std::process::ExitCode;

use crate::parameters::ServerParameters;
use crate::server;

use super::egress;

pub(super) fn build_command() -> Command {
    Command::new("server")
        .about("Server side daemon")
        .visible_alias("s")
        .arg(
            Arg::new("windows-daemon")
                .long("windows-daemon")
                .action(ArgAction::SetTrue)
                .hide(true)
                .global(true),
        )
        .subcommand(Command::new("start").about("Start the server daemon"))
        .subcommand(Command::new("restart").about("Restart the server daemon"))
        .subcommand(Command::new("stop").about("Stop the server daemon"))
        .subcommand(Command::new("ls").about("Show server sync root and generation state"))
}

#[must_use]
pub fn dispatch(sub: &ArgMatches, server: &ServerParameters) -> ExitCode {
    match sub.subcommand() {
        Some(("start", _)) => egress(server::start(server)),
        Some(("restart", _)) => egress(server::restart(server)),
        Some(("stop", _)) => egress(server::stop()),
        Some(("ls", _)) => egress(server::ls(server)),
        _ => {
            fancy_error!("unknown subcommand");
            ExitCode::FAILURE
        }
    }
}

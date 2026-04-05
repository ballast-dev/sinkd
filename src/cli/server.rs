use clap::ArgMatches;
use std::process::ExitCode;

use crate::parameters::ServerParameters;
use crate::server;

use super::common::{egress, print_subcommand_help};

#[must_use]
pub(super) fn dispatch(sub: &ArgMatches, server: &ServerParameters) -> ExitCode {
    match sub.subcommand() {
        Some(("start", _)) => egress(server::start(server)),
        Some(("restart", _)) => egress(server::restart(server)),
        Some(("stop", _)) => egress(server::stop()),
        _ => print_subcommand_help("server"),
    }
}

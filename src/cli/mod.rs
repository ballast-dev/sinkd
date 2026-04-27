use crate::outcome::Outcome;
use clap::{Arg, ArgAction, Command};
use std::process::ExitCode;

pub mod client;
pub mod init;
pub mod server;

pub fn egress<T>(outcome: Outcome<T>) -> ExitCode {
    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            fancy_error!("ERROR: {}", e);
            ExitCode::FAILURE
        }
    }
}

#[must_use]
pub fn build() -> Command {
    let verbose_arg = Arg::new("verbose")
        .short('v')
        .action(ArgAction::Count)
        .help("log verbosity: v=error, vv=warn (default), vvv=info, vvvv=debug")
        .global(true);

    let debug_arg = Arg::new("debug")
        .short('d')
        .long("debug")
        .action(ArgAction::Count)
        .help("debug logs under /tmp/sinkd; -dd also enables Zenoh/IPC crate logs in the file")
        .global(true);

    Command::new("sinkd")
        .about("Sync daemon: `sinkd client …` or `sinkd server …`.")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(verbose_arg)
        .arg(debug_arg)
        .subcommand_required(true)
        .subcommand(client::build_command())
        .subcommand(server::build_command())
}

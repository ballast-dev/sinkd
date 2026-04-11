use clap::{Arg, ArgAction, Command};
use std::process::ExitCode;
use crate::outcome::Outcome;

pub mod client;
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
    let windows_daemon = Arg::new("windows-daemon")
        .long("windows-daemon")
        .action(ArgAction::SetTrue)
        .hide(true)
        .global(true);

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

    let client_state_dir_arg = Arg::new("client-state-dir")
        .long("client-state-dir")
        .value_name("DIR")
        .num_args(1)
        .global(true)
        .hide(true)
        .help("store client_id / ack state under DIR (tests / multi-instance)");

    Command::new("sinkd")
        .about("Sync daemon: `sinkd client …` or `sinkd server …`.")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(windows_daemon)
        .arg(verbose_arg)
        .arg(debug_arg)
        .arg(client_state_dir_arg)
        .subcommand(client::build_command())
        .subcommand(server::build_command())
}

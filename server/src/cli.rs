use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{path::PathBuf, process::ExitCode};

use sinkd_core::{fancy_error, outcome::Outcome};

use crate::{params::ServerParameters, server};

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
pub fn build_command() -> Command {
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

    Command::new("sinkd-srv")
        .about("Sync server: receive client sync traffic via Zenoh.")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(verbose_arg)
        .arg(debug_arg)
        .arg(
            Arg::new("windows-daemon")
                .long("windows-daemon")
                .action(ArgAction::SetTrue)
                .hide(true)
                .global(true),
        )
        .subcommand_required(true)
        .subcommand(Command::new("start").about("Start the server daemon"))
        .subcommand(Command::new("restart").about("Restart the server daemon"))
        .subcommand(Command::new("stop").about("Stop the server daemon"))
        .subcommand(Command::new("ls").about("Show server sync root and generation state"))
        .subcommand(
            Command::new("init")
                .about("Scaffold the system config file from the embedded template")
                .arg(
                    Arg::new("users")
                        .long("users")
                        .value_name("LIST")
                        .num_args(1)
                        .required(true)
                        .help("Comma-separated list of users to register (e.g. alice,bob)"),
                )
                .arg(
                    Arg::new("server-addr")
                        .long("server-addr")
                        .value_name("HOST")
                        .num_args(1)
                        .default_value("0.0.0.0")
                        .help("Value to write into `server_addr` (informational on the server side)"),
                )
                .arg(
                    Arg::new("config")
                        .long("config")
                        .value_name("PATH")
                        .num_args(1)
                        .help("Override system config target path (default: /etc/sinkd.conf, /opt/sinkd/sinkd.conf on macOS)"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .action(ArgAction::SetTrue)
                        .help("Overwrite existing config file"),
                ),
        )
}

#[must_use]
pub fn dispatch(matches: &ArgMatches, server_params: &ServerParameters) -> ExitCode {
    match matches.subcommand() {
        Some(("start", _)) => egress(server::start(server_params)),
        Some(("restart", _)) => egress(server::restart(server_params)),
        Some(("stop", _)) => egress(server::stop()),
        Some(("ls", _)) => egress(server::ls(server_params)),
        Some(("init", s)) => egress(run_init(s)),
        _ => {
            fancy_error!("unknown subcommand");
            ExitCode::FAILURE
        }
    }
}

fn run_init(sub: &ArgMatches) -> Outcome<()> {
    let users_csv = sub
        .get_one::<String>("users")
        .ok_or_else(|| "server init: --users is required".to_string())?;
    let users: Vec<String> = users_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if users.is_empty() {
        return sinkd_core::bad!("server init: --users must contain at least one name");
    }
    let server_addr = sub
        .get_one::<String>("server-addr")
        .map_or("0.0.0.0", String::as_str);
    let force = sub.get_flag("force");

    let target = sub
        .get_one::<String>("config")
        .map_or_else(default_system_target, PathBuf::from);

    sinkd_core::init::init_server_config(&target, server_addr, &users, force)
}

#[must_use]
fn default_system_target() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/opt/sinkd/sinkd.conf")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\ProgramData\sinkd\sinkd.conf")
    } else {
        PathBuf::from("/etc/sinkd.conf")
    }
}

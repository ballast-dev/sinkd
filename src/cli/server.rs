use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{path::PathBuf, process::ExitCode};

use crate::outcome::Outcome;
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
pub fn dispatch(sub: &ArgMatches, server: &ServerParameters) -> ExitCode {
    match sub.subcommand() {
        Some(("start", _)) => egress(server::start(server)),
        Some(("restart", _)) => egress(server::restart(server)),
        Some(("stop", _)) => egress(server::stop()),
        Some(("ls", _)) => egress(server::ls(server)),
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
        .ok_or("server init: --users is required")?;
    let users: Vec<String> = users_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if users.is_empty() {
        return bad!("server init: --users must contain at least one name");
    }
    let server_addr = sub
        .get_one::<String>("server-addr")
        .map_or("0.0.0.0", String::as_str);
    let force = sub.get_flag("force");

    let target = sub
        .get_one::<String>("config")
        .map_or_else(default_system_target, PathBuf::from);

    server::init_config(&target, server_addr, &users, force)
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

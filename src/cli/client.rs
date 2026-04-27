use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use super::egress;
use crate::client;
use crate::outcome::Outcome;
use crate::parameters::ClientParameters;

#[allow(clippy::too_many_lines)]
pub(super) fn build_command() -> Command {
    let share_arg = Arg::new("share")
        .short('S')
        .long("share")
        .value_name("SHARE")
        .num_args(1)
        .action(ArgAction::Append)
        .help("file or folder path for multiple users");

    let path_arg = Arg::new("path")
        .value_name("PATH")
        .num_args(1..)
        .help("file or folder path");

    let user_arg = Arg::new("user")
        .value_name("USER")
        .num_args(1..)
        .help("username");

    Command::new("client")
        .about("Client side daemon")
        .visible_alias("c")
        .arg(
            Arg::new("windows-daemon")
                .long("windows-daemon")
                .action(ArgAction::SetTrue)
                .hide(true)
                .global(true),
        )
        .arg(
            Arg::new("client-state-dir")
                .long("client-state-dir")
                .value_name("DIR")
                .num_args(1)
                .hide(true)
                .global(true)
                .help("store client_id / ack state under DIR (tests / multi-instance)"),
        )
        .arg(
            Arg::new("system-config")
                .help("system TOML (overrides default path)")
                .long_help("overrides default system config path")
                .short('s')
                .long("sys-cfg")
                .num_args(1)
                .global(true),
        )
        .arg(
            Arg::new("user-configs")
                .help("user TOML(s) (overrides default path(s))")
                .long_help("overrides default user config path(s)")
                .short('u')
                .long("usr-cfg")
                .num_args(1)
                .action(ArgAction::Append)
                .global(true),
        )
        .subcommand(
            Command::new("add")
                .about("Add PATH(s) to watch list")
                .args([&share_arg, &path_arg]),
        )
        .subcommand(
            Command::new("rm")
                .visible_alias("remove")
                .about("Remove PATH(s) from watch list")
                .args([&share_arg, &path_arg]),
        )
        .subcommand(
            Command::new("adduser")
                .about("Add USER(s) to watch list")
                .arg(&user_arg),
        )
        .subcommand(
            Command::new("rmuser")
                .about("Remove USER(s) from watch list")
                .arg(&user_arg),
        )
        .subcommand(
            Command::new("ls")
                .visible_alias("list")
                .about("List watched files for PATH(s)")
                .arg(&path_arg),
        )
        .subcommand(Command::new("log").about("Show client log output"))
        .subcommand(Command::new("start").about("Start the client daemon"))
        .subcommand(Command::new("restart").about("Restart the client daemon"))
        .subcommand(Command::new("stop").about("Stop the client daemon"))
        .subcommand(
            Command::new("init")
                .about("Scaffold system + user config files from templates")
                .arg(
                    Arg::new("server-addr")
                        .long("server-addr")
                        .value_name("HOST")
                        .num_args(1)
                        .required(true)
                        .help("Sync target host (rsync destination)"),
                )
                .arg(
                    Arg::new("user")
                        .long("user")
                        .value_name("NAME")
                        .num_args(1)
                        .required(true)
                        .help("Owning user for this client (also added to system `users` list)"),
                )
                .arg(
                    Arg::new("watch")
                        .long("watch")
                        .value_name("ABS_PATH")
                        .num_args(1)
                        .required(true)
                        .help("Absolute path to the watch anchor"),
                )
                .arg(
                    Arg::new("interval")
                        .long("interval")
                        .value_name("SECS")
                        .num_args(1)
                        .default_value("1")
                        .help("Polling interval for the anchor (seconds)"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .action(ArgAction::SetTrue)
                        .help("Overwrite existing config files"),
                ),
        )
}

fn check_path_exists(p: &str) -> bool {
    let p = Path::new(p);
    if p.exists() {
        return true;
    }
    crate::fancy::println(
        &format!("path doesn't exist: {}", p.display()),
        crate::fancy::Attrs::Bold,
        crate::fancy::Colors::Red,
    );
    false
}

fn collect_share_user_paths(submatches: &ArgMatches) -> (Vec<&String>, Vec<&String>) {
    let share_paths = submatches
        .get_many::<String>("share")
        .map(|shares| shares.filter(|p| check_path_exists(p)).collect())
        .unwrap_or_default();
    let user_paths = submatches
        .get_many::<String>("path")
        .map(|paths| paths.filter(|p| check_path_exists(p)).collect())
        .unwrap_or_default();
    (share_paths, user_paths)
}

#[must_use]
pub fn dispatch(sub: &ArgMatches, params: &ClientParameters) -> ExitCode {
    match sub.subcommand() {
        Some(("start", _)) => egress(client::start(params)),
        Some(("restart", _)) => egress(client::restart(params)),
        Some(("stop", _)) => egress(client::stop(params)),
        Some(("add", s)) => {
            let (sp, up) = collect_share_user_paths(s);
            egress(client::add(params, &sp, &up))
        }
        Some(("rm", s)) => {
            let (sp, up) = collect_share_user_paths(s);
            egress(client::rm(params, &sp, &up))
        }
        Some(("adduser", s)) => egress(client::adduser(params, s.get_many::<String>("user"))),
        Some(("rmuser", s)) => egress(client::rmuser(params, s.get_many::<String>("user"))),
        Some(("ls", s)) => {
            let paths = s
                .get_many::<String>("path")
                .map(|ps| ps.filter(|p| check_path_exists(p)).collect());
            egress(client::ls(params, paths))
        }
        Some(("log", _)) => egress(client::log(params)),
        Some(("init", s)) => egress(run_init(s, params)),
        _ => {
            fancy_error!("unknown subcommand");
            ExitCode::FAILURE
        }
    }
}

fn run_init(sub: &ArgMatches, params: &ClientParameters) -> Outcome<()> {
    let server_addr = sub
        .get_one::<String>("server-addr")
        .map(String::as_str)
        .ok_or("client init: --server-addr is required")?;
    let user = sub
        .get_one::<String>("user")
        .map(String::as_str)
        .ok_or("client init: --user is required")?;
    let watch_arg = sub
        .get_one::<String>("watch")
        .ok_or("client init: --watch is required")?;
    let interval: u64 = sub
        .get_one::<String>("interval")
        .map_or("1", String::as_str)
        .parse()
        .map_err(|e| format!("client init: --interval must be an integer: {e}"))?;
    let force = sub.get_flag("force");

    let watch = PathBuf::from(watch_arg);
    let sys_target: PathBuf = params.system_config.as_ref().as_path().to_path_buf();
    let user_target = client::default_user_config_target();
    let users = vec![user.to_string()];

    client::init_config(
        &sys_target,
        &user_target,
        server_addr,
        &users,
        &watch,
        interval,
        force,
    )
}

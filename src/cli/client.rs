use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{path::Path, process::ExitCode};

use crate::client;
use crate::parameters::ClientParameters;
use super::egress;

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
        .arg(Arg::new("system-config")
            .help("system TOML (overrides default path)")
            .long_help("overrides default system config path")
            .short('s')
            .long("sys-cfg")
            .num_args(1)
            .global(true))
        .arg(Arg::new("user-configs")
            .help("user TOML(s) (overrides default path(s))")
            .long_help("overrides default user config path(s)")
            .short('u')
            .long("usr-cfg")
            .num_args(1)
            .action(ArgAction::Append)
            .global(true))
        .subcommand(Command::new("add")
            .about("Add PATH(s) to watch list")
            .args([&share_arg, &path_arg])
        )
        .subcommand(Command::new("rm")
            .visible_alias("remove")
            .about("Remove PATH(s) from watch list")
            .args([&share_arg, &path_arg]))
        .subcommand(Command::new("adduser")
            .about("Add USER(s) to watch list")
            .arg(&user_arg))
        .subcommand(Command::new("rmuser")
            .about("Remove USER(s) from watch list")
            .arg(&user_arg))
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
        _ => {
            fancy_error!("unknown subcommand");
            ExitCode::FAILURE
        }
    }
}

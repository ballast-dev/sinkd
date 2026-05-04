use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{
    path::Path,
    process::ExitCode,
};

use sinkd_core::{fancy_error, outcome::Outcome};

use crate::{client, parameters::ClientParameters};

pub fn egress<T>(outcome: Outcome<T>) -> ExitCode {
    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            fancy_error!("ERROR: {}", e);
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)]
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

    Command::new("sinkd")
        .about("Deployable Cloud: client side")
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
                .long_help("overrides default system config path (/etc/sinkd.conf on Unix, %APPDATA%\\sinkd\\sinkd.system.conf on Windows)")
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
        .subcommand_required(true)
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
                        .help(
                            "Owning user for this client (also added to system `users` list); default: USER or USERNAME",
                        ),
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
    sinkd_core::fancy::println(
        &format!("path doesn't exist: {}", p.display()),
        sinkd_core::fancy::Attrs::Bold,
        sinkd_core::fancy::Colors::Red,
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
pub fn dispatch(matches: &ArgMatches, parameters: &ClientParameters) -> ExitCode {
    match matches.subcommand() {
        Some(("start", _)) => egress(client::start(parameters)),
        Some(("restart", _)) => egress(client::restart(parameters)),
        Some(("stop", _)) => egress(client::stop(parameters)),
        Some(("add", s)) => {
            let (sp, up) = collect_share_user_paths(s);
            egress(client::add(parameters, &sp, &up))
        }
        Some(("rm", s)) => {
            let (sp, up) = collect_share_user_paths(s);
            egress(client::rm(parameters, &sp, &up))
        }
        Some(("adduser", s)) => egress(client::adduser(parameters, s.get_many::<String>("user"))),
        Some(("rmuser", s)) => egress(client::rmuser(parameters, s.get_many::<String>("user"))),
        Some(("ls", s)) => {
            let paths = s
                .get_many::<String>("path")
                .map(|ps| ps.filter(|p| check_path_exists(p)).collect());
            egress(client::ls(parameters, paths))
        }
        Some(("log", _)) => egress(client::log(parameters)),
        Some(("init", s)) => egress(crate::init::run(s, parameters)),
        _ => {
            fancy_error!("unknown subcommand");
            ExitCode::FAILURE
        }
    }
}

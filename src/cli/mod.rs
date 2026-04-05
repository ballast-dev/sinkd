//! Command-line definition and dispatch.

mod client;
mod common;
mod dispatch;
mod server;

// pub use build::build_sinkd;
// pub use dispatch::{dispatch_sinkd_matches, run_sinkd};

use clap::{Arg, ArgAction, Command};

fn client_command() -> Command {
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
        .subcommand(Command::new("ls")
            .visible_alias("list")
            .about("List watched files for PATH(s)")
            .arg(&path_arg))
        .subcommand(Command::new("log").about("Show client log output"))
        .subcommand(Command::new("start").about("Start the client daemon"))
        .subcommand(Command::new("restart").about("Restart the client daemon"))
        .subcommand(Command::new("stop").about("Stop the client daemon"))
}

fn server_command() -> Command {
    Command::new("server")
        .about("Server side daemon")
        .visible_alias("s")
        .subcommand(Command::new("start").about("Start the server daemon"))
        .subcommand(Command::new("restart").about("Restart the server daemon"))
        .subcommand(Command::new("stop").about("Stop the server daemon"))
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
        .subcommand(client_command())
        .subcommand(server_command())
}

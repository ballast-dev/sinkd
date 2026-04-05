use clap::{Arg, ArgAction, Command};

fn with_lifecycle(cmd: Command, role: &'static str) -> Command {
    ["start", "restart", "stop"]
        .into_iter()
        .fold(cmd, |cmd, sub| {
            cmd.subcommand(Command::new(sub).about(format!("{sub} the {role} daemon")))
        })
}

fn client_command(
    system_config_arg: &Arg,
    user_configs_arg: &Arg,
    share_arg: &Arg,
    path_arg: &Arg,
    user_arg: &Arg,
) -> Command {
    let add_cmd = Command::new("add")
        .about("Add PATH(s) to watch list")
        .args([share_arg, path_arg]);

    let rm_cmd = Command::new("rm")
        .visible_alias("remove")
        .about("Remove PATH(s) from watch list")
        .args([share_arg, path_arg]);

    let adduser_cmd = Command::new("adduser")
        .about("Add USER(s) to watch list")
        .arg(user_arg);

    let rmuser_cmd = Command::new("rmuser")
        .about("Remove USER(s) from watch list")
        .arg(user_arg);

    let ls_cmd = Command::new("ls")
        .visible_alias("list")
        .about("List watched files for PATH(s), or use --server")
        .arg(path_arg)
        .arg(
            Arg::new("server")
                .long("server")
                .action(ArgAction::SetTrue)
                .conflicts_with("path")
                .help("list server-side paths (stub message)"),
        );

    with_lifecycle(
        Command::new("client")
            .about("Client daemon and config (anchors, users, ls, log). TOML: -s / -u.")
            .visible_alias("c")
            .args([system_config_arg, user_configs_arg]),
        "client",
    )
    .subcommand(ls_cmd)
    .subcommand(add_cmd)
    .subcommand(rm_cmd)
    .subcommand(adduser_cmd)
    .subcommand(rmuser_cmd)
    .subcommand(Command::new("log").about("Show client log output"))
}

#[must_use]
pub fn build_sinkd() -> Command {
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

    let system_config_arg = Arg::new("system-config")
        .help("system TOML (overrides default path)")
        .long_help("overrides default system config path")
        .short('s')
        .long("sys-cfg")
        .num_args(1)
        .global(true);

    let user_configs_arg = Arg::new("user-configs")
        .help("user TOML(s) (overrides default path(s))")
        .long_help("overrides default user config path(s)")
        .short('u')
        .long("usr-cfg")
        .num_args(1)
        .action(ArgAction::Append)
        .global(true);

    let windows_daemon = Arg::new("windows-daemon")
        .long("windows-daemon")
        .action(ArgAction::SetTrue)
        .hide(true)
        .global(true);

    let client = client_command(
        &system_config_arg,
        &user_configs_arg,
        &share_arg,
        &path_arg,
        &user_arg,
    );
    let server = with_lifecycle(
        Command::new("server")
            .about("Server daemon only (no client TOML).")
            .visible_alias("s"),
        "server",
    );

    Command::new("sinkd")
        .about("Sync daemon: `sinkd client …` or `sinkd server …`.")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(windows_daemon)
        .arg(
            Arg::new("verbose")
                .short('v')
                .action(ArgAction::Count)
                .help("log verbosity: v=error, vv=warn (default), vvv=info, vvvv=debug")
                .global(true),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .action(ArgAction::Count)
                .help(
                    "debug logs under /tmp/sinkd; -dd also enables Zenoh/IPC crate logs in the file",
                )
                .global(true),
        )
        .arg(
            Arg::new("client-state-dir")
                .long("client-state-dir")
                .value_name("DIR")
                .num_args(1)
                .global(true)
                .hide(true)
                .help("store client_id / ack state under DIR (tests / multi-instance)"),
        )
        .subcommand(client)
        .subcommand(server)
}

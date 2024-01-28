use clap::{Arg, ArgAction, Command};

#[rustfmt::skip]
pub fn build_sinkd() -> Command {
    // composable args
    let share_arg = Arg::new("share")
        .short('s')
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
        .help("system configuration file to use")
        //? long help is '--help' versus '-h'
        .long_help("providing this flag will override default")
        .short('s')
        .long("sys-cfg")
        .num_args(1)
        .global(true)
        .default_value("/etc/sinkd.conf");

    let user_configs_arg = Arg::new("user-configs")
        .help("user configuration file(s) to use")
        //? long help is '--help' versus '-h'
        .long_help("providing this flag will override default")
        .short('u')
        .long("usr-cfg")
        .num_args(1..)  // might need to reconsider this 
        .value_delimiter(',')
        .action(ArgAction::Append)
        .global(true)
        .default_value("~/.config/sinkdrc");

    // composable commands

    let add_cmd = Command::new("add")
        .about("Add PATH(s) to watch list")
        .args([&share_arg,  &path_arg]);

    let rm_cmd = Command::new("rm")
        .visible_alias("remove")
        .about("Removes PATH(s) from watch list")
        .args([&share_arg,  &path_arg]);

    let adduser_cmd = Command::new("adduser")
        .about("Add USER(s) to watch list")
        .arg(&user_arg);

    let rmuser_cmd = Command::new("rmuser")
        .about("Removes PATH(s) from watch list")
        .arg(&user_arg);


    // app 
    Command::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::new("verbose")
            .short('v')
            .action(ArgAction::Count)
            .help("verbosity, corresponds to log level, default='vv'=warn\nv=error,vv=warn,vvv=info,vvvv=debug")
            .global(true)
        )
        .arg(Arg::new("debug")
            .short('d')
            .long("debug")
            .action(ArgAction::SetTrue)
            .help("log files to /tmp, log-level set to debug")
            .global(true)
        )
        .subcommand(Command::new("server")
            .about("manage sinkd server")
            .visible_alias("s")
            .subcommand(Command::new("start")
                .about("start the server daemon")
            )
            .subcommand(Command::new("restart")
                .about("restart the server daemon")
            )
            .subcommand(Command::new("stop")
                .about("stop the server daemon")
            )
        )
        .subcommand(Command::new("client")
            .about("manage sinkd client")
            .visible_alias("c")
            // global goes down stream
            .args([&system_config_arg, &user_configs_arg])
            .subcommand(Command::new("start")
                .about("start the client daemon")
            )
            .subcommand(Command::new("restart")
                .about("restart the client daemon")
            )
            .subcommand(Command::new("stop")
                .about("stop the client daemon")
            )
        )
        .subcommands([&add_cmd, &rm_cmd, &adduser_cmd, &rmuser_cmd])
        .subcommand(Command::new("ls")
            .visible_alias("list")
            .about("List currently watched files from given PATH")
            .arg(&path_arg)
            .arg(Arg::new("server")
                .help("show tracked files on server")
                .conflicts_with("path")
            )
        )
        .subcommand(Command::new("log")
            .about("show logs")
        )
}

extern crate clap;
extern crate notify;
extern crate regex;
extern crate toml;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate libc;
extern crate rpassword;

mod client;
mod config;
mod protocol;
mod server;
mod shiplog;
mod sinkd;
mod test;
mod utils;

use clap::*;

pub fn build_sinkd() -> Command<'static> {
    Command::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(Command::new("init")
            .display_order(1)
            .about("Setup sinkd on client or server")
            .arg(Arg::new("CLIENT")
                .long("client")
                .takes_value(false)
                .help("initialize sinkd daemon on client")
            )
            .arg(Arg::new("SERVER")
                .long("server")
                .takes_value(false)
                .conflicts_with("CLIENT")
                .help("initialize sinkd daemon on server")
            )
            .override_usage("sinkd init [--client | --server]")
        )
        .subcommand(Command::new("add")
            .about("Adds PATH to watch list\nlets sinkd become 'aware' of file or folder location provided")
            .arg(Arg::new("SHARE")
                .short('s')
                .long("share")
                .help("add watch for multiple users")
            )
            .arg(Arg::new("PATH")
                .required(true)
                .multiple_occurrences(true) // CAREFUL: this will consume other arguments
                .help("sinkd starts watching path")
            )
            .override_usage("sinkd add FILE [FILE..]")
        )
        .subcommand(Command::new("ls")
            .alias("list")
            .about("List currently watched files from given PATH")
            .arg(Arg::new("PATHS")
                // need to revisit, should user have explicit control
                // possible -r flag for recursive 
                .required(false)
                .multiple_occurrences(true) // CAREFUL: this will consume other arguments
                .help("list watched files and directories")
            )
            .override_usage("sinkd ls [PATH..]")
        )
        .subcommand(Command::new("rm")
            .alias("remove")
            .about("Removes PATH from list of watched directories")
            .arg(Arg::new("PATH")
                .required(true)
                .multiple_occurrences(true) // CAREFUL: this will consume other arguments
            )
            .override_usage("sinkd rm PATH")
        )
        .subcommand(Command::new("start")
            .about("Starts the daemon")
            .override_usage("sinkd start [--client | --server]")
            .arg(Arg::new("CLIENT")
                .short('c')
                .long("client")
                .takes_value(false)
                .help("start sinkd in client mode")
            )
            .arg(Arg::new("SERVER")
                .short('s')
                .long("server")
                .takes_value(false)
                .conflicts_with("CLIENT")
                .help("start sinkd in server mode")
            )
        )
        .subcommand(Command::new("stop")
            .about("Stops daemon")
        )
        .subcommand(Command::new("restart")
            .about("Restarts sinkd, reloading configuration")
        )
        .subcommand(Command::new("log")
            .about("test out logging")
        )
        .arg(Arg::new("verbose")
            .short('v')
            .multiple_occurrences(true)
            .help("verbose output")
        )
}

#[allow(dead_code)]
fn main() {
    println!("Running sinkd at {}", utils::get_timestamp("%Y%m%d-%T"));

    // mqtt::listen();
    // std::process::exit(0);
    let matches = build_sinkd().get_matches();
    let mut verbosity: u8 = 0;
    match matches.occurrences_of("verbose") {
        1 => verbosity = 1,
        2 => verbosity = 2,
        3 => verbosity = 3,
        _ => (),
    }

    match matches.subcommand() {
        // ("init", Some(argv)) => {
        //     if argv.is_present("SERVER") {
        //         init::server(verbosity);
        //     } else {
        //         init::client(verbosity);
        //     }
        // },
        Some(("add", argv)) => {
            for path in argv.values_of("PATH").unwrap() {
                if std::path::Path::new(path).exists() {
                    sinkd::add(path);
                } else {
                    println!("'{}' does not exist", path);
                }
            }
        },
        Some(("ls", path)) => { sinkd::list(&path.values_of_lossy("PATHS").unwrap_or_else(|| {vec![]}));},
        Some(("rm", _)) => { sinkd::remove();},
        Some(("start", argv)) => { 
            if argv.is_present("SERVER") {
                // server::start(verbosity);
            } else if !client::start(verbosity) { 
                println!("unable to start client, take a look: {}", utils::LOG_PATH)
            }
        },
        Some(("stop",    _)) => { sinkd::stop();},
        Some(("restart", _)) => { sinkd::restart()},
        Some(("log",     _)) => { sinkd::log()},
        _ => {}
    }
}

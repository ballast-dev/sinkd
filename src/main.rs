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

mod config;
mod client;
mod utils;
mod shiplog;
mod sinkd;
mod init;
mod server;
mod test;
mod protocol;

use clap::*;

pub fn build_sinkd() -> App<'static, 'static> {
    App::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(App::new("init")
            .display_order(1)
            .about("Setup sinkd on client or server")
            .arg(Arg::with_name("CLIENT")
                .long("client")
                .takes_value(false)
                .help("initialize sinkd daemon on client")
            )
            .arg(Arg::with_name("SERVER")
                .long("server")
                .takes_value(false)
                .conflicts_with("CLIENT")
                .help("initialize sinkd daemon on server")
            )
            .usage("sinkd init [--client | --server]")
        )
        .subcommand(App::new("add")
            .about("Adds PATH to watch list")
            .arg(Arg::with_name("SHARE")
                .short("s")
                .long("share")
                .help("add watch for multiple users")
            )
            .arg(Arg::with_name("PATH")
                .required(true)
                .multiple(true) // CAREFUL: this will consume other arguments
                .help("sinkd starts watching path")
            )
            .usage("usage: sinkd add FILE [FILE..]\n\
                lets sinkd become 'aware' of file or folder location provided")
        )
        .subcommand(App::new("ls")
            .alias("list")
            .about("List currently watched files from given PATH")
            .arg(Arg::with_name("PATHS")
                // need to revisit, should user have explicit control
                // possible -r flag for recursive 
                .required(false)
                .multiple(true) // CAREFUL: this will consume other arguments
                .help("list watched files and directories")
            )
            .help("usage: sinkd ls [PATH | PATH ...]")
        )
        .subcommand(App::new("rm")
            .alias("remove")
            .about("Removes PATH from list of watched directories")
            .arg(Arg::with_name("PATH")
                .required(true)
                .multiple(true) // CAREFUL: this will consume other arguments
            )
            .help("usage: sinkd rm PATH")
        )
        .subcommand(App::new("start")
            .about("Starts the daemon")
        )
        .subcommand(App::new("stop")
            .about("Stops daemon")
        )
        .subcommand(App::new("restart")
            .about("Restarts sinkd, reloading configuration")
        )
        .subcommand(App::new("log")
            .about("test out logging")
        )
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
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
        _ => ()
    }

    match matches.subcommand() {
        ("init", Some(sub_match)) => {
            if sub_match.is_present("SERVER") {
                init::server(verbosity);
            } else {
                init::client(verbosity);
            }
        },
        ("add", Some(sub_match)) => {
            for path in sub_match.values_of("PATH").unwrap() {
                if std::path::Path::new(path).exists() {
                    sinkd::add(path);
                } else {
                    println!("'{}' does not exist", path);
                }
            }
        },
        ("ls",      Some(path)) => { sinkd::list(&path.values_of_lossy("PATHS").unwrap());},
        ("rm",      Some(_)) => { sinkd::remove();},
        ("start",   Some(_)) => { sinkd::start();},
        ("stop",    Some(_)) => { sinkd::stop();},
        ("restart", Some(_)) => { sinkd::restart()},
        ("log",     Some(_)) => { sinkd::log()},
        _ => {}
    }
}

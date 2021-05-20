extern crate clap;
extern crate notify;
extern crate regex;
extern crate toml;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate libc;
extern crate paho_mqtt;
extern crate rpassword;


mod config;
mod client;
mod utils;
mod shiplog;
mod sinkd;
mod mqtt;

use clap::*;

pub fn build_sinkd() -> App<'static, 'static> {
    App::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(App::new("init")
            .display_order(1)
            .about("Setup sinkd on client and server")
            .arg(Arg::with_name("CLIENT")
                .long("client")
                .help("initialize sinkd daemon on client")
            )
            .arg(Arg::with_name("SERVER")
                .long("server")
                .help("initialize sinkd daemon on server")
            )
            .usage("sinkd init [--client | --server]")
        )
        .subcommand(App::new("add")
            .alias("anchor")
            .about("Adds PATH to watch list")
            .arg(Arg::with_name("PATH")
                .required(true)
                .multiple(true) // CAREFUL: this will consume other arguments
                .help("sinkd starts watching path")
            )
            .usage("usage: sinkd add FILE [FILE..]\n\
                lets sinkd become 'aware' of file or folder location provided")
        )
        .subcommand(App::new("adduser")
            .alias("hire")
            .about("Add USER to watch")
            .arg(Arg::with_name("USER")
                .required(true)
                .multiple(true) // CAREFUL: this will consume other arguments
                .help("sinkd adduser USER")
            )
            .usage("usage: sinkd adduser USER [USER..]")
        )
        .subcommand(App::new("ls")
            .alias("list")
            .alias("parley")
            .about("List currently watched files from given PATH")
            .arg(Arg::with_name("PATH")
                // need to revisit, should user have explicit control
                // possible -r flag for recursive 
                .required(false)
                .multiple(true) // CAREFUL: this will consume other arguments
                .help("list watched files and directories")
            )
            .help("usage: sinkd ls [PATH]")
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
        .subcommand(App::new("rmuser")
            .about("Removes USER from watch")
            .arg(Arg::with_name("USER")
                .required(true)
            )
            .help("usage: sinkd rmuser USER")
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
        // .help("sinkd \n help and shit\n----boom-sauce")
}


#[allow(dead_code)]
fn main() {

    println!("Running sinkd at {}", utils::get_timestamp("%Y%m%d-%T"));

    shiplog::ShipLog::init();
    // mqtt::listen();
    // std::process::exit(0);
    let matches = build_sinkd().get_matches();
    let mut verbosity: u8 = 0;
    match matches.occurrences_of("verbose") {
        1 => verbosity = 1, // informationation
        2 => verbosity = 2, // 
        3 => verbosity = 3,
        _ => ()
    }

    match matches.subcommand() {
        ("init", Some(sub_match)) => {
            if let Some(host) = sub_match.value_of("SERVER") {
                //? need to setup up mqtt on server 
                //? server will send out a broadcast every ten seconds 
                //? of any updates
                sinkd::setup_keys(verbosity, &host);
            } if let Some(host) = sub_match.value_of("CLIENT") {
                //? client will rsync forward
                //? mqtt subscribe for updates from server 
                sinkd::setup_server(verbosity, &host);
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
        ("adduser", Some(_)) => {
            sinkd::adduser(matches.values_of("USER").unwrap().collect());
        },
        ("ls",      Some(_)) => { sinkd::list();},
        ("rm",      Some(_)) => { sinkd::remove();},
        ("start",   Some(_)) => { sinkd::start();},
        ("stop",    Some(_)) => { sinkd::stop();},
        ("restart", Some(_)) => { sinkd::restart()},
        ("log",     Some(_)) => { sinkd::log()},
        _ => {
            use utils::*;
            print_fancyln("deploy the anchor matey!", Attrs::INVERSE, Colors::YELLOW); 
            println!("invalid command, try -h for options"); 
        }
    }
}

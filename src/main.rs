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


mod rigging;
mod tradewind;
mod ropework;
mod shiplog;
mod sinkd;
mod mqtt;

use clap::*;

pub fn build_sinkd() -> App<'static, 'static> {
    App::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(App::new("setup")
            .display_order(1)
            .visible_alias("rig")
            .about("Setup sinkd on local and remote")
            .arg(Arg::with_name("SETUP_KEYS")
                .short("k")
                .long("keys")
                .value_name("HOSTNAME")
                .number_of_values(1)
                .takes_value(true)
                .help("setup ssh-keys between local and remote machine")
            )
            .arg(Arg::with_name("SETUP_SERVER")
                .short("s")
                .long("server")
                .value_name("HOSTNAME")
                .number_of_values(1)
                .takes_value(true)
                .help("setup config and start daemon on server")
                // .help("HOSTNAME of remote machine can be IPADDR too")
            )
            .usage("usage: sinkd setup [--keys, --server] HOSTNAME")
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
            .alias("brig")
            .about("Removes PATH from list of watched directories")
            .arg(Arg::with_name("PATH")
                .required(true)
                .multiple(true) // CAREFUL: this will consume other arguments
            )
            .help("usage: sinkd rm PATH")
        )
        .subcommand(App::new("rmuser")
            .alias("fire")
            .about("Removes USER from watch")
            .arg(Arg::with_name("USER")
                .required(true)
            )
            .help("usage: sinkd rmuser USER")
        )
        .subcommand(App::new("start")
            .alias("deploy")
            .about("Starts the daemon")
        )
        .subcommand(App::new("stop")
            .alias("snag")
            .about("Stops daemon")
        )
        .subcommand(App::new("restart")
            .alias("oilskins")
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

    println!("Running sinkd at {}", ropework::get_timestamp("%Y%m%d-%T"));

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
        ("setup", Some(sub_match)) => {
            if let Some(host) = sub_match.value_of("SETUP_KEYS") {
                sinkd::setup_keys(verbosity, &host);
            } if let Some(host) = sub_match.value_of("SETUP_SERVER") {
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
            use ropework::*;
            print_fancyln("deploy the anchor matey!", Attrs::INVERSE, Colors::YELLOW); 
            println!("invalid command, try -h for options"); 
        }
    }
}

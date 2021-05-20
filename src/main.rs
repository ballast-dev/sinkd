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
mod setup;
mod server;

use clap::*;
use config::SysConfig;

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

#[test]
fn config() {
    let sys_config: SysConfig = match config::get_sys_config() {
        Ok(sys_config) => { sys_config },
        Err(err) => {
            panic!("/etc/sinkd.conf ERROR {}", err);
        }
    };
    for user in sys_config.users {
        let _cfg = format!("/home/{}/.config/sinkd.conf", user);
        match std::fs::read_to_string(&_cfg) {
            Ok(_) => {
                println!("going to user {}", &user);
                match config::get_user_config(&user) {
                    Ok(_) => {},
                    Err(e) => {
                        panic!("Error {}, {}", &_cfg, e)
                    }
                }
            },
            Err(_) => { 
                eprintln!("configuration not loaded for {}", &user) 
            }
        }
    }
}

#[test]
fn dummy() {
    println!("sucess?");
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
                setup::setup_keys(verbosity, &host);
            } if let Some(host) = sub_match.value_of("CLIENT") {
                //? client will rsync forward
                //? mqtt subscribe for updates from server 
                setup::setup_server(verbosity, &host);
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

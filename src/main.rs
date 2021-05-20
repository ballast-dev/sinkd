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
        .subcommand(App::new("add")
            .alias("anchor")
            .about("Adds PATH to watch list")
            .arg(Arg::with_name("PATH")
                .required(true)
                .multiple(true) // CAREFUL: this will consume other arguments
                .help("sinkd starts watching path")
            )
            .help("usage: sinkd add FILE [FILE..]\n\
                lets sinkd become 'aware' of file or folder location provided")
        )
        .subcommand(App::new("adduser")
            .alias("hire")
            .about("Add USER to watch")
            .arg(Arg::with_name("USER [USER..]")
                .required(true)
                .multiple(true) // CAREFUL: this will consume other arguments
                .help("sinkd adduser USER")
            )
            .help("usage: sinkd adduser USER")
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
}


#[allow(dead_code)]
fn main() {

    // rigging::TimeStamp::show();
    shiplog::ShipLog::init();
    // mqtt::listen();
    // std::process::exit(0);

    let matches = build_sinkd().get_matches();
    
    if let Some(matches) = matches.subcommand_matches("add") {
        for path in matches.values_of("PATH").unwrap() {
            if std::path::Path::new(path).exists() {
                sinkd::add(path);
            } else {
                println!("'{}' does not exist", path);
            }
        }
    }
    
    if let Some(matches) = matches.subcommand_matches("adduser") {
        sinkd::adduser(matches.values_of("USER").unwrap().collect());
    }

    if let Some(_) = matches.subcommand_matches("ls")      { sinkd::list(); }
    if let Some(_) = matches.subcommand_matches("rm")      { sinkd::remove(); }
    if let Some(_) = matches.subcommand_matches("start")   { sinkd::start(); }
    if let Some(_) = matches.subcommand_matches("stop")    { sinkd::stop(); }
    if let Some(_) = matches.subcommand_matches("restart") { sinkd::restart(); }
    if let Some(_) = matches.subcommand_matches("log")     { sinkd::log(); }
}

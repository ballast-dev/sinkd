extern crate clap;
extern crate notify;
extern crate regex;
extern crate toml;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;


mod rigging;
mod tradewind;
mod ropework;
mod shiplog;
mod sinkd;

use clap::*;

pub fn build_sinkd() -> App<'static, 'static> {
    App::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        // TODO revisit, maybe a better approach
        .arg(Arg::with_name("daemon")  // for debugging
            .short("d")
            .long("daemon")
            .hidden(true) // reentry point to spawn daemon for barge
        )
        .subcommand(App::new("harbor")
        // harbor by default will print off configuration
            .about("Control the harbor daemon (on server)")
            .arg(Arg::with_name("dock")
                .short("d")
                .long("dock")
                .help("Generates configuration and starts harbor daemon")
            )
            .arg(Arg::with_name("start")
                .long("start")
                .help("starts sinkd harbor (server)")
            )       
            .arg(Arg::with_name("stop")
                .long("stop")
                .help("stops sinkd harbor (server)")
            )       
            .help("All harbor commands should be invoked on the 'server'\ncontrol the server side daemon (hoster of files)")
        )
        .subcommand(App::new("add")
            .about("Adds PATH to watch list")
            .arg(Arg::with_name("PATH")
                .required(true)
                .help("sinkd starts watching path")
            )
            .help("usage: sinkd add FILE [OPTIONS]\n\
                lets sinkd become 'aware' of file or folder location provided")
        )
        .subcommand(App::new("adduser")
            .about("Add USER to watch")
            .arg(Arg::with_name("[USER, ...]")
                .required(true)
                .help("sinkd adduser USER")
            )
            .help("usage: sinkd adduser USER")
        )
        .subcommand(App::new("ls")
            .alias("list")
            .about("List currently watched files from given PATH")
            .arg(Arg::with_name("PATH")
                .required(false)
                .help("list watched files and directories")
            )
            .help("usage: sinkd ls [PATH]")
        )
        .subcommand(App::new("rm")
            .alias("remove")
            .about("Removes PATH from list of watched directories")
            .arg(Arg::with_name("PATH")
                .required(true)
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
}


#[allow(dead_code)]
fn main() {

    shiplog::ShipLog::init();

    let matches = build_sinkd().get_matches();
    
    if let Some(matches) = matches.subcommand_matches("add") {
        let path = String::from(matches.value_of("PATH").unwrap());
        
        if std::path::Path::new(&path[..]).exists() {
            sinkd::add(path);
        } else {
            println!("'{}' does not exist", path);
        }
    }
    
    if let Some(matches) = matches.subcommand_matches("adduser") {
        sinkd::adduser(matches.values_of("USER").unwrap().collect());
    }

    if let Some(_) = matches.subcommand_matches("ls") {
        sinkd::list();
    }

    if let Some(_) = matches.subcommand_matches("rm") {
        sinkd::remove();
    }

    if let Some(_) = matches.subcommand_matches("start") {
        sinkd::start();
    }

    if let Some(_) = matches.subcommand_matches("stop") {
        sinkd::stop();
    }
    
    if let Some(_) = matches.subcommand_matches("restart") {
        sinkd::restart();
    }

    if let Some(_) = matches.subcommand_matches("log") {
        sinkd::log();
    }

}

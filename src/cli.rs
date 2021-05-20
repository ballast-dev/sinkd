/*
 * Command Line Interface
 */
use crate::daemon::barge::Barge;
use crate::daemon::harbor::Harbor;
use clap::*;

//
// D E P L O Y 
//

/**
 * essentially the harbor and barge are two separate folder locations
 * harbor and barge can live on the same machine
 * just need to make sure one is not already deployed
 */

pub fn deploy(ip: &str) -> bool {
    // starts the daemon remotely (If not already deployed)
    // ssh into another machine
    // and start the sinkd daemon
    println!("Deployed to {}!", ip);
    return true // able to start daemon on another computer
}
//
// E N D   D E P L O Y 
//


pub enum DaemonType {
    Barge,
    Harbor,
}
// deploy
// 1. add
// 1. adduser
// 1. ls
// 1. rm
// 1. rmuser
// 1. start
// 1. stop

pub fn build_cli() -> App<'static, 'static> {
    App::new("sinkd")
        .version(env!("CARGO_PKG_VERSION"))
        .about("deployable cloud")
        // NOTE: possibly have user install sinkd on server...
        //       other option is to push configs over to server to enable harbor
        //       ease of use is critical, vote to push configs
        // .subcommand(SubCommand::with_name("deploy")
        //     .about("enable daemon, pushes edits to given IP")
        //     .arg(Arg::with_name("IP")
        //         .required(true)
        //         .help("IPv4 address, ssh access required")
        //     )
        //     .help("sets up sinkd server on remote computer")
        // )
        .subcommand(SubCommand::with_name("add")
            .about("adds path to watch list")
            .arg(Arg::with_name("PATH")
                .required(true)
                .help("sinkd starts watching path")
            )
            .help("usage: sinkd add FILE [OPTIONS]\n\
                lets sinkd become 'aware' of file or folder location provided")
        )
        .subcommand(SubCommand::with_name("adduser")
            .about("add user to watch")
            .arg(Arg::with_name("[USER, ...]")
            .required(true)
            .help("sinkd adduser USER")
        )
        .subcommand(SubCommand::with_name("ls")
            .alias("list")
            .arg(Arg::with_name("PATH")
                .required(false)
                .help("list watched files and directories")
            )
            .help("list currently watched files from given PATH")
        )
        .subcommand(SubCommand::with_name("rm")
            .alias("remove")
            .about("removes PATH from list of watched directories")
            .arg(Arg::with_name("PATH")
                .required(true)
            )
            .help("usage: sinkd rm PATH")
        )
        .subcommand(SubCommand::with_name("rmuser")
            .about("removes user from watch")
            .arg(Arg::with_name("USER")
                .required(true)
            )
            .help("usage: sinkd rmuser USER")
        )
        .subcommand(SubCommand::with_name("start")
            .about("starts the daemon")
        )
        .subcommand(SubCommand::with_name("stop")
            .about("stops daemon")
        )
        .subcommand(SubCommand::with_name("hoist")
            .about("stops and starts the daemon (rescans config)")
        )
    )
}


pub fn add(daemon_type: DaemonType, file: String) -> bool {

    match daemon_type {
        DaemonType::Barge => {
            println!("appending '{}' to watch files", file);   
            let mut barge = Barge::new(); 
            barge.anchor(file, 1, Vec::new()); 
            return true; // able to watch directory
        },
        DaemonType::Harbor => {
            // stuff for server
            println!("anchor in for harbor");
            return true;
        }
    }
}

pub fn adduser(users: Vec<&str>) {
    println!("add {:?} to list of users who have permission to watch the directory",
             users)
}

pub fn add(my_str: &str) {
    
}
pub fn list() {
    // list all current files loaded
    println!("print out list of all watched folders")
}

pub fn stop() {
    println!("stopping daemon")
}

pub fn refresh() {
    println!("refreshing")
}

pub fn remove() {
    println!("remove files and folders")
}


// the same daemon should run on both machines ( in the same place )
pub fn underway(daemon_type: DaemonType) {
    match daemon_type {
        DaemonType::Barge => println!("starting barge"),
        DaemonType::Harbor => println!("starting harbor"),
    }
}

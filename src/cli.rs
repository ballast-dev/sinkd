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

pub fn build_cli() -> App<'static, 'static> {
    App::new("sinkd")
        .version("0.1.0")
        .about("deployable cloud, drop anchor and go")
        .subcommand(SubCommand::with_name("deploy")
            .about("enable daemon, pushes edits to given IP")
            .arg(Arg::with_name("IP")
                .required(true)
                .help("IPv4 address, ssh access required")
            )
            .help("sets up sinkd server on remote computer")
        )
        .subcommand(SubCommand::with_name("add")
            .about("adds folder/file to watch list")
            .arg(Arg::with_name("FILE")
                .required(true)
                .help("sinkd starts watching folder/file")
            )
            .help("usage: sinkd anchor [OPTION] FILE\n\
                lets sinkd become 'aware' of file or folder location provided")
        )
        .subcommand(SubCommand::with_name("ls")
            .alias("list")
            .arg(Arg::with_name("PATH")
                .required(false)
                .help("list watched files and directories from supplied PATH")
            )
            .help("list currently watched directories")
        )
        .subcommand(SubCommand::with_name("rm")
            .alias("remove")
            .about("removes PATH from list of watched directories")
            .arg(Arg::with_name("PATH")
                .required(true)
            )
            .help("usage: sinkd remove PATH")
        )
        // daemon should be started and refreshed automagically
        .subcommand(SubCommand::with_name("stop")
            .about("stops daemon")
        )
        .subcommand(SubCommand::with_name("refresh")
            .about("stops and starts the daemon (updates config)")
        )
        .subcommand(SubCommand::with_name("adduser")
            .about("add user to watch")
            .arg(Arg::with_name("[USER...]")
            .required(true)
            .help("sinkd recruit USER DIRECTORY")
        )
    )
}

/** localhost file syncing separate daemons */
pub fn anchor(daemon_type: DaemonType, file: String) -> bool {

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

pub fn recruit(users: Vec<&str>) {
    println!("add {:?} to list of users who have permission to watch the directory",
             users)
}

pub fn add() {
    println!("add folder to watch list")
}

pub fn list() {
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
    // 1 parse config 
    // 2 put running rust code
    match daemon_type {
        DaemonType::Barge => println!("starting barge"),
        DaemonType::Harbor => println!("starting harbor"),
    }
}

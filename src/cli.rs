/*
 * Command Line Interface
 */
use crate::daemon::barge::Barge;
use crate::daemon::harbor::Harbor;
use clap::*;

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
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("daemon")
            .short("d")
            .long("daemon")
            .hidden(true)
        )
        .subcommand(App::new("add")
            .about("adds path to watch list")
            .arg(Arg::with_name("PATH")
                .required(true)
                .help("sinkd starts watching path")
            )
            .help("usage: sinkd add FILE [OPTIONS]\n\
                lets sinkd become 'aware' of file or folder location provided")
        )
        .subcommand(App::new("adduser")
            .about("add user to watch")
            .arg(Arg::with_name("[USER, ...]")
                .required(true)
                .help("sinkd adduser USER")
            )
            .help("usage: sinkd adduser USER")
        )
        .subcommand(App::new("ls")
            .alias("list")
            .arg(Arg::with_name("PATH")
                .required(false)
                .help("list watched files and directories")
            )
            .help("list currently watched files from given PATH")
        )
        .subcommand(App::new("rm")
            .alias("remove")
            .about("removes PATH from list of watched directories")
            .arg(Arg::with_name("PATH")
                .required(true)
            )
            .help("usage: sinkd rm PATH")
        )
        .subcommand(App::new("rmuser")
            .about("removes user from watch")
            .arg(Arg::with_name("USER")
                .required(true)
            )
            .help("usage: sinkd rmuser USER")
        )
        .subcommand(App::new("start")
            .about("starts the daemon")
        )
        .subcommand(App::new("stop")
            .about("stops daemon")
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

pub fn list() {
    // list all current files loaded
    println!("print out list of all watched folders")
}

pub fn start() {
    std::process::Command::new("sinkd")
                           .arg("--daemon")
                           .arg("&") // spawn in background
                           .output()
                           .expect("ERROR couldn't start daemon");
}

pub fn daemon() {
    let mut barge = Barge::new();
    barge.daemon(); // never returns
}

pub fn stop() {
    println!("stopping daemon");
    // need to keep pid of barge process in separate file
    std::process::Command::new("kill")
                           .arg("-15")
                           .arg("pid of process")
                           .output()
                           .expect("ERROR couldn't kill daemon");
}

pub fn restart() {
    // stop the running daemon
    // spawn the running daemon
    // refresh the configuration
    // NOTE: this should be called after every configuration change, maybe manual at first?
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

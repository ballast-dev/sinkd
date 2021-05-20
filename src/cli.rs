/*
 * Command Line Interface
 */
use crate::daemon::barge::Barge;
use crate::daemon::harbor::Harbor;


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
    println!("Deployed!");
    return true // able to start daemon on another computer
}
//
// E N D   D E P L O Y 
//


pub enum DaemonType {
    Barge,
    Harbor,
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

pub fn recruit(user: &str) {
    println!("add user to list of users who have permission to watch the directory")
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

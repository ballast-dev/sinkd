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

pub enum DaemonType {
    Barge,
    Harbor,
}
pub fn deploy(ip: &str) -> bool {
    // starts the daemon remotely (If not already deployed)
    // ssh into another machine
    // and start the sinkd daemon
    return true // able to start daemon on another computer
}
//
// E N D   D E P L O Y 
//

/** localhost file syncing separate daemons */
pub fn anchor(file: &str) -> bool {
    println!("appending '{}' to watch files", file);    
    return true // able to watch directory
}

pub fn add_user(user: &str) {
}

// insider function to enable local daemon
fn init() {
    // starts up the daemon locally
    // if not initialized previously
}
pub fn list() {
    // print out all sinkd folders
}

pub fn status() {
    // ncurses TUI that shows all folders with progress bar on the bottom
    // repeat for every char "-\|/#"
}

pub fn start(daemon_type: DaemonType) {
    match daemon_type {
        Barge => Barge::start(),
        Harbor => Harbor::start(),
    }    
}

pub fn stop() {

}

// calls stop then start 
pub fn restart() {

}
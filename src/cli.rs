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
pub fn anchor(file: &str) -> bool {
    println!("appending '{}' to watch files", file);    
    return true // able to watch directory
}

pub fn timoneer(user: &str) {

}

pub fn parley() {
    // print out all sinkd folders
}

pub fn brig() {
    // ncurses TUI that shows all folders with progress bar on the bottom
    // repeat for every char "-\|/#"
}

// the same daemon should run on both machines ( in the same place )
pub fn underway(daemon_type: DaemonType ) {
    // 1 parse config 
    // 2 put running rust code
    match daemon_type {
        DaemonType::Barge => println!("starting barge"),
        DaemonType::Harbor => println!("starting harbor"),
    }
}

// stops the server
pub fn snag() {
}

// calls stop then start (restart)
pub fn oilskins() {

}
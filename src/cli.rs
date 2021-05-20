/*
 * Command Line Interface
 */
use crate::daemon::barge::*;
/**
 * essentially the harbor and barge are two separate folder locations
 * harbor and barge can live on the same machine
 * just need to make sure one is not already deployed
 */
pub fn deploy(ip: &str) -> bool {
    // ssh into another machine
    // and start the sinkd daemon
    return true // able to start daemon on another computer
}

pub fn anchor(file: &str) -> bool {
    println!("appending '{}' to watch files", file);    
    return true // able to watch directory
}

pub fn add_user(user: &str) {
  
}

pub fn start() {
    let barge = Barge::new();
    barge.parse_conf();
}

pub fn stop() {

}

// calls stop then start 
pub fn restart() {

}
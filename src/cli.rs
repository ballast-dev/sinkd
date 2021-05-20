/**
 * Command Line Interface
  - `sinkd deploy IP` creates harbor on server
  - `sinkd anchor DIRECTORY` creates DIRECTORY on harbor (server file location)
    - loads DIRECTORY in sinkd.json (top-level)
    - possibility of multiple directories inside harbor folder
  - `sinkd start` starts daemon
  - `sinkd stop` stops daemon
  - `sinkd restart` restarts daemon
 * 
 */


pub fn deploy() -> bool {
    return true // able to start daemon on another computer
}

pub fn anchor(file: &str) -> bool{
    return true // able to watch directory
}

pub fn start() {
    
}

pub fn stop() {

}

// calls stop then start 
pub fn restart() {

}
/**
 * B A R G E 
 *
 * --- 
 *
 * Client side of sinkd
 * will hook into anchor and fail if anchor is not there
 * 
 * be able to start and stop daemon 
 * this binary will be invoked in usr directory i.e. /usr/local/bin
 */

extern crate notify;

use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::env;
use std::path::PathBuf;



pub struct AnchorPoint {
    // directory to watch
    path: PathBuf,
    interval: u32,  // cycle time to check changes
}

impl AnchorPoint {

    pub fn from(path: &str, interval: u32) -> AnchorPoint {
        let path_buf = PathBuf::from(path);
        return AnchorPoint {
            path: path_buf,
            interval, 
        }
    }

    pub fn set_interval(&mut self, interval: u32) {
        self.interval = interval;
    }
}



/**
 * 
 */
fn watch(dir_to_watch: &str, interval: u32) -> bool {
    let anchor_point = AnchorPoint::from(dir_to_watch, interval);
    // need to write to a config file
    return true;
}


/**
 * upon edit of configuration restart the daemon
 */
pub fn start_daemon() {

    // need to start daemon from config file
    // need to parse json
    // set up json within /etc/sinkd.json

    // serde already does json

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    let result = watcher.watch("path/to/watch", RecursiveMode::Recursive);
    match result {
        Err(_) => {
            println!("path not found, unable to set watcher");
            std::process::exit(1);
        },

        Ok(_) => ()
    }

    loop {
        match rx.recv() {
           Ok(event) => println!("{:?}", event),
           Err(e) => println!("watch error: {:?}", e),
        }
    }
}
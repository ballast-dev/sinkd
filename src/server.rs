//    ____                    
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/   
#![allow(unused_imports)]

use std::{process, sync::mpsc, thread};
use crate::protocol;


enum State {
    SYNCHING,
    READY,
}

// `sinkd start` starts up the client daemon
// `sinkd start -s,--server` will start up the server daemon

// - (pkgr) mkdir /srv/sinkd 
// - (pkgr) chmod 2770 /srv/sinkd (for setgid, not recursive for user permissions to retain)
// - (pkgr) cd /srv/sinkd/ && umask 5007
// - (pkgr) create systemd unit file with appropriate flags
// - (pkgr) enable service 
// - (pkgr) start service >> which calls sinkd::server::start()


pub fn start() {
    // first subscribe to `sinkd/status` 
    // let mut mqtt_client: protocol::MqttClient;
    // match protocol::MqttClient::new(Some("localhost"), dispatch) {
    //     Ok(mc) => mqtt_client = mc,
    //     Err(err_str) => {
    //         error!("unable to initialize mqtt: {}", err_str);
    //         std::process::exit(2);
    //     }
    // }

    // let (synch_tx, synch_rx): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();
    
    // keep things alive between threads by calling outside of scope

    // let mqtt_thread = thread::spawn(move || {
    //     if let Err(e) = mqtt_entry(synch_tx) {
    //         panic!("mqtt thread unable to start");
    //     }
    // });

    // let synch_thread = thread::spawn(move || {
    //     if let Err(e) = synch_entry(synch_rx) {
    //         panic!("synch thread unable to start");
    //     }
    // });
    
    // if let Err(_) = mqtt_thread.join() {
    //     error!("Client watch thread error!");
    //     std::process::exit(1);
    // }
    // if let Err(_) = synch_thread.join() {
    //     error!("Client synch thread error!");
    //     std::process::exit(1);
    // }




}

// fn synch_entry(synch_rx: mpsc::Receiver<String>) -> Result<(), String> {
//     Ok(())
// }
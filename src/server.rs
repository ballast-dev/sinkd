//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/   
#![allow(unused_imports)]

use paho_mqtt as mqtt;
use std::{
    process, 
    sync::mpsc, 
    thread
};
use crate::shiplog;
enum State {
    SYNCHING,
    READY,
}

// `sinkd start` starts up the client daemon
// `sinkd start -s,--server` will start up the server daemon (maybe check for client daemon?)

// - (pkgr) mkdir /srv/sinkd
// - (pkgr) chmod 2770 /srv/sinkd (for setgid, not recursive for user permissions to retain)
// - (pkgr) cd /srv/sinkd/ && umask 5007
// - (pkgr) create systemd unit file with appropriate flags
// - (pkgr) enable service
// - (pkgr) start service >> which calls sinkd::server::start()

// ## Server
// 1. setup mqtt daemon on this machine
// 1. mqtt subscribe to `sinkd/update`
// 1. mqtt publish to `sinkd/status`
// 1. *listening thread*
//     - receive packets from clients
//     - update broadcast to current status
//     - add request to `synch_queue`
// 1. *synching thread*
//     - process `sync_queue`
//     - sets state to `SYNCHING` when processing request
//     - once done with all requests set state to `READY`
// 1. *broadcast thread*
//     - push out messsages with current status
//     - interval (every 5 secs) of status

pub fn start(verbosity: u8, clear_logs: bool) {
    if let Err(e) = shiplog::init(clear_logs) {
        eprintln!("{}", e);
        process::exit(2);
    }

    debug!("server:start >> initial");
    if let Err(_) = process::Command::new("mosquitto").arg("-d").spawn() {
        eprintln!("mosquitto not installed or not in PATH");
        return;
    }

    debug!("server:start >> spawning channel");
    let (msg_tx, msg_rx): (mpsc::Sender<mqtt::Message>, mpsc::Receiver<mqtt::Message>) =
        mpsc::channel();

    let mqtt_thread = thread::spawn(move || mqtt_entry(msg_tx));

    let synch_thread = thread::spawn(move || synch_entry(msg_rx));

    if let Err(_) = mqtt_thread.join() {
        error!("server:mqtt_thread join error!");
        process::exit(1);
    }
    if let Err(_) = synch_thread.join() {
        error!("server::synch_thread join error!");
        process::exit(1);
    }
}

fn dispatch(msg: &Option<mqtt::Message>) {
    if let Some(msg) = msg {
        let payload = std::str::from_utf8(msg.payload()).unwrap();
        debug!("topic: {}\tpayload: {}", msg.topic(), payload);
    } else {
        error!("malformed mqtt message");
    }
}

fn mqtt_entry(tx: mpsc::Sender<mqtt::Message>) -> ! {
    // TODO: Read from config
     let opts = mqtt::CreateOptionsBuilder::new()
                        .server_uri("tcp://localhost:1883")
                        .client_id("sinkd_server")
                        .finalize();
    
    match mqtt::Client::new(opts) {
        Err(e) => {
            error!("FATAL could not create the mqtt server client: {}", e);
            process::exit(2);
        }
        Ok(cli) => {
            // Initialize the consumer before connecting
            let msg_rx = cli.start_consuming();
            let lwt = mqtt::Message::new("sinkd/lost_conn", "sinkd server client lost connection", 1);
            let conn_opts = mqtt::ConnectOptionsBuilder::new()
                .keep_alive_interval(std::time::Duration::from_secs(300)) // 5 mins should be enough
                .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
                .clean_session(true)
                .will_message(lwt)
                .finalize();

            match cli.connect(conn_opts) {
                Ok(rsp) => {
                    if let Some(conn_rsp) = rsp.connect_response() {
                        debug!(
                            "Connected to: '{}' with MQTT version {}",
                            conn_rsp.server_uri, conn_rsp.mqtt_version
                        );
                        if conn_rsp.session_present {
                            debug!("  w/ client session already present on broker.");
                        } else {
                            // Register subscriptions on the server
                            debug!("Subscribing to topics with requested QoS: {:?}...", mqtt::QOS_0);
                            if let Err(e) = cli.subscribe("sinkd/status", mqtt::QOS_0) {
                                error!("server:mqtt_entry >> could not subscribe to sinkd/status {}", e);
                            }
            
                            // cli.subscribe_many(&subscriptions, &qos)
                            //     .and_then(|rsp| {
                            //         rsp.subscribe_many_response()
                            //             .ok_or(mqtt::Error::General("Bad response"))
                            //     })
                            //     .and_then(|vqos| {
                            //         println!("QoS granted: {:?}", vqos);
                            //         Ok(())
                            //     })
                            //     .unwrap_or_else(|err| {
                            //         println!("Error subscribing to topics: {:?}", err);
                            //         cli.disconnect(None).unwrap();
                            //         process::exit(1);
                            //     });
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "FATAL client could not connect to localhost:1883, is mosquitto -d running? {}", e
                    );
                    process::exit(2);
                }
            }
            // start processing messages
            loop {
                if let Ok(msg) = msg_rx.recv() {
                    tx.send(msg.unwrap()).unwrap();
                }
            }
        }
    }
}

fn synch_entry(rx: mpsc::Receiver<mqtt::Message>) -> ! {
    loop {
        match rx.recv() {
            Err(e) => {
                error!("server:synch_entry hangup on reciever?: {}", e);
            }
            Ok(msg) => {
                debug!("server:synch_entry >> {}", msg);
                // let rsync_result = process::Command::new("rsync")
                //     .arg("-atR") // archive, timestamps, relative
                //     .arg("--delete")
                //     // TODO: to add --exclude [list of folders] from config
                //     // .arg(&src_path)
                //     // .arg(&dest_path)
                //     .spawn();

                // match rsync_result {
                //     Err(x) => {
                //         error!("{:?}", x);
                //     }
                //     Ok(_) => {
                //         info!(
                //             "DID IT>> Called rsync",
                //             // &src_path.display(),
                //             // &dest_path
                //         );
                //     }
                // }
            }
        }
    }
}

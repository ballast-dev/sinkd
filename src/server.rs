//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/   
#![allow(unused_imports)]

use crate::protocol;
use paho_mqtt as mqtt;
use std::{process, sync::mpsc, thread};

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

pub fn start() {
    if let Err(_) = std::process::Command::new("mosquitto").arg("-d").spawn() {
        eprintln!("mosquitto not installed or not in PATH");
        return;
    }

    let (msg_tx, msg_rx): (mpsc::Sender<mqtt::Message>, mpsc::Receiver<mqtt::Message>) =
        mpsc::channel();

    // keep things alive between threads by calling outside of scope

    let mqtt_thread = thread::spawn(move || mqtt_entry(msg_tx));

    let synch_thread = thread::spawn(move || synch_entry(msg_rx));

    if let Err(_) = mqtt_thread.join() {
        error!("server:mqtt_thread join error!");
        std::process::exit(1);
    }
    if let Err(_) = synch_thread.join() {
        error!("server::synch_thread join error!");
        std::process::exit(1);
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
    match mqtt::AsyncClient::new("tcp://localhost:1883") {
        Err(e) => {
            error!("FATAL could not create the mqtt server client: {}", e);
            std::process::exit(2);
        }
        Ok(mut cli) => {
            let lwt = mqtt::Message::new("sinkd/lost_conn", "Async subscriber lost connection", 1);
            let conn_opts = mqtt::ConnectOptionsBuilder::new()
                .keep_alive_interval(std::time::Duration::from_secs(300)) // 5 mins should be enough
                .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
                .clean_session(true)
                .will_message(lwt)
                .finalize();
            cli.connect(conn_opts.clone());
            cli.subscribe("sinkd/status", mqtt::QOS_0);
            if cli.is_connected() {
                let msg_rx = cli.start_consuming();
                loop {
                    if let Ok(msg) = msg_rx.try_recv() {
                        tx.send(msg.unwrap()).unwrap();
                    } else {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
            } else {
                // TODO: create a fatal! macro
                error!(
                    "FATAL client could not connect to localhost:1883, is mosquitto -d running?"
                );
                std::process::exit(2);
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
                let rsync_result = std::process::Command::new("rsync")
                    .arg("-atR") // archive, timestamps, relative
                    .arg("--delete")
                    // TODO: to add --exclude [list of folders] from config
                    // .arg(&src_path)
                    // .arg(&dest_path)
                    .spawn();

                match rsync_result {
                    Err(x) => {
                        error!("{:?}", x);
                    }
                    Ok(_) => {
                        info!(
                            "DID IT>> Called rsync",
                            // &src_path.display(),
                            // &dest_path
                        );
                    }
                }
            }
        }
    }
}

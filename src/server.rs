//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
#![allow(unused_imports)]
extern crate serde;
use crate::{ipc, shiplog};
use paho_mqtt as mqtt;
use std::{
    path::PathBuf,
    process,
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub fn start(verbosity: u8, clear_logs: bool) -> Result<(), String> {
    shiplog::init(clear_logs)?;
    debug!("server:start >> mosquitto daemon");
    if let Err(spawn_error) = process::Command::new("mosquitto").arg("-d").spawn() {
        return Err(format!(
            "Is mosquitto installed and in path? >> {}",
            spawn_error.to_string()
        ));
    }

    debug!("server:start >> spawning channel");
    let (msg_tx, msg_rx): (mpsc::Sender<mqtt::Message>, mpsc::Receiver<mqtt::Message>) =
        mpsc::channel();

    let exit_cond = Arc::new(Mutex::new(false));
    let exit_cond2 = Arc::clone(&exit_cond);

    let bcast_cycle = Arc::new(Mutex::new(0));
    let incr_cycle = Arc::clone(&bcast_cycle);

    // error handling must be done within the threads
    let mqtt_thread = thread::spawn(move || {
        if let Err(err) = mqtt_entry(msg_tx, exit_cond, bcast_cycle, verbosity) {
            error!("{}", err);
        }
    });
    let synch_thread = thread::spawn(move || {
        if let Err(err) = synch_entry(msg_rx, exit_cond2, incr_cycle, verbosity) {
            error!("{}", err);
        }
    });

    if let Err(mqtt_thread_err) = mqtt_thread.join() {
        error!("server:mqtt_thread join error! >> {:?}", mqtt_thread_err);
        process::exit(1);
    }
    if let Err(synch_thread_err) = synch_thread.join() {
        error!("server::synch_thread join error! >> {:?}", synch_thread_err);
        process::exit(1);
    }
    Ok(())
}

fn dispatch(msg: &Option<mqtt::Message>) {
    if let Some(msg) = msg {
        let payload = std::str::from_utf8(msg.payload()).unwrap();
        debug!("topic: {}\tpayload: {}", msg.topic(), payload);
    } else {
        error!("malformed mqtt message");
    }
}

//? This thread is to ensure no lost messages from mqtt
fn mqtt_entry(
    tx: mpsc::Sender<mqtt::Message>,
    exit_cond: Arc<Mutex<bool>>,
    cycle: Arc<Mutex<i32>>,
    verbosity: u8,
) -> Result<(), mqtt::Error> {
    let (mqtt_client, mqtt_rx) =
        ipc::MqttClient::new(Some("localhost"), &["sinkd/clients"], "sinkd/server")?;

    match mqtt::Client::new("tcp://localhost:1883") {
        Err(e) => {
            error!("FATAL: unable to create mqtt server client: {}", e);
            process::exit(2);
        }
        Ok(cli) => {
            // Initialize the consumer before connecting
            let msg_rx = cli.start_consuming();
            let lwt =
                mqtt::Message::new("sinkd/lost_conn", "sinkd server client lost connection", 1);
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
                            if let Err(e) = cli.subscribe("sinkd/update", mqtt::QOS_0) {
                                error!("unable to subscribe to sinkd/status {}", e);
                            }
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

            loop {
                if let Ok(msg) = msg_rx.try_recv() {
                    tx.send(msg.unwrap()).unwrap();
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    publish(&cli, &cycle.lock().unwrap().to_string());
                }
            }
        }
    }
}

fn synch_entry(
    rx: mpsc::Receiver<mqtt::Message>,
    exit_cond: Arc<Mutex<bool>>,
    cycle: Arc<Mutex<i32>>,
    verbosity: u8,
) -> Result<(), String> {
    loop {
        match rx.recv() {
            Err(e) => {
                error!("server:synch_entry hangup on reciever?: {}", e);
            }
            Ok(msg) => {
                debug!("server:synch_entry >> {}", msg);

                //? RSYNC options to consider
                // --delete-excluded (also delete excluded files)
                // --max-size=SIZE (limit size of transfers)
                // --exclude

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
                // info!(
                //     "DID IT>> Called rsync",
                //     // &src_path.display(),
                //     // &dest_path
                // );
                let mut num = cycle.lock().unwrap();
                *num += 1;

                //     }
                // }
            }
        }
    }
}

fn publish<'a>(mqtt_client: &mqtt::Client, msg: &'a str) {
    if let Err(e) = mqtt_client.publish(mqtt::Message::new("sinkd/status", msg, mqtt::QOS_0)) {
        error!("server:publish >> {}", e);
    }
}

// TODO: move to it's own file
fn fire_rsync(hostname: &String, src_path: &PathBuf) {
    // debug!("username: {}, hostname: {}, path: {}", username, hostname, path.display());

    // Agnostic pathing allows sinkd not to care about user folder structure
    let dest_path: String;
    if hostname.starts_with('/') {
        // TODO: packager should set up folder '/srv/sinkd'
        dest_path = String::from("/srv/sinkd/");
    } else {
        // user permissions should persist regardless
        dest_path = format!("sinkd@{}:/srv/sinkd/", &hostname);
    }

    let rsync_result = std::process::Command::new("rsync")
        .arg("-atR") // archive, timestamps, relative
        .arg("--delete")
        // TODO: to add --exclude [list of folders] from config
        .arg(&src_path)
        .arg(&dest_path)
        .spawn();

    match rsync_result {
        Err(x) => {
            error!("{:?}", x);
        }
        Ok(_) => {
            info!(
                "DID IT>> Called rsync src:{}  ->  dest:{}",
                &src_path.display(),
                &dest_path
            );
        }
    }
}

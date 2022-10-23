//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
#![allow(unused_imports)]
extern crate serde;
use crate::{ipc, shiplog, utils};
use paho_mqtt as mqtt;
use std::{
    path::PathBuf,
    process,
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub fn start(verbosity: u8, clear_logs: bool) -> Result<(), String> {
    shiplog::init(clear_logs)?;
    start_mosquitto()?; // always spawns on localhost

    let (msg_tx, msg_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
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

//? This command will not spawn new instances
//? if mosquitto already active.
pub fn start_mosquitto() -> Result<(), String> {
    debug!("server:start >> mosquitto daemon");
    if let Err(spawn_error) = process::Command::new("mosquitto").arg("-d").spawn() {
        return Err(format!(
            "Is mosquitto installed and in path? >> {}",
            spawn_error.to_string()
        ));
    }
    Ok(())
}

//? This thread is to ensure no lost messages from mqtt
fn mqtt_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    exit_cond: Arc<Mutex<bool>>,
    cycle: Arc<Mutex<i32>>,
    verbosity: u8,
) -> Result<(), mqtt::Error> {
    let (mqtt_client, mqtt_rx) =
        ipc::MqttClient::new(Some("localhost"), &["sinkd/clients"], "sinkd/server")?;

    match mqtt::Client::new("tcp://localhost:1883") {
        Err(e) => {
            utils::fatal(&exit_cond);
            return Err(mqtt::Error::GeneralString(format!(
                "FATAL: unable to create mqtt server client: {}",
                e
            )));
        }
        Ok(cli) => {
            // Initialize the consumer before connecting
            let mqtt_rx = cli.start_consuming();
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
                    utils::fatal(&exit_cond);
                    return Err(mqtt::Error::GeneralString(
                        format!("FATAL client could not connect to localhost:1883, is mosquitto -d running? {}", e)
                    ));
                }
            }

            loop {
                if utils::exited(&exit_cond) {
                    return Err(mqtt::Error::General(
                        "server>> synch thread exited, aborting mqtt thread",
                    ));
                }
                match mqtt_rx.try_recv() {
                    Ok(msg) => {
                        // ! process mqtt messages
                        let payload = ipc::decode(msg.unwrap().payload())?;
                        synch_tx.send(payload).unwrap();
                    }
                    Err(err) => match err {
                        crossbeam::channel::TryRecvError::Empty => {
                            std::thread::sleep(std::time::Duration::from_millis(1500));
                            debug!("server>> mqtt loop...")
                        }
                        crossbeam::channel::TryRecvError::Disconnected => {
                            utils::fatal(&exit_cond);
                            return Err(mqtt::Error::General(
                                "server>> mqtt_rx channel disconnected",
                            ));
                        }
                    },
                }
            }
        }
    }
}

/// The engine behind sinkd is rsync
/// With mqtt messages that are relevant invoke this and mirror current client
/// to this server
fn synch_entry(
    synch_rx: mpsc::Receiver<ipc::Payload>,
    exit_cond: Arc<Mutex<bool>>,
    cycle: Arc<Mutex<i32>>,
    verbosity: u8,
) -> Result<(), String> {
    loop {
        if utils::exited(&exit_cond) {
            return Err(String::from(
                "server>> mqtt_thread exited, aborting synch thread",
            ));
        }
        match synch_rx.recv() {
            // blocking to
            Err(e) => {
                error!("server:synch_entry hangup on reciever?: {}", e);
            }
            Ok(payload) => {
                debug!("server:synch_entry >> got message from mqtt_thread!");

                utils::rsync(payload);
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
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn publish<'a>(mqtt_client: &mqtt::Client, msg: &'a str) {
    if let Err(e) = mqtt_client.publish(mqtt::Message::new("sinkd/status", msg, mqtt::QOS_0)) {
        error!("server:publish >> {}", e);
    }
}

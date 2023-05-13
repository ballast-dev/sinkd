//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
#![allow(unused_imports)]
extern crate serde;
use crate::{
    ipc,
    outcome::{err_msg, Outcome},
    shiplog, utils::{self, Parameters},
};
use paho_mqtt as mqtt;
use std::{
    path::PathBuf,
    process,
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub fn start(params: &Parameters) -> Outcome<()> {
    shiplog::init(params)?;
    start_mosquitto()?; // always spawns on localhost

    let (msg_tx, msg_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
        mpsc::channel();

    let exit_cond = Arc::new(Mutex::new(false));
    let exit_cond2 = Arc::clone(&exit_cond);

    let bcast_cycle = Arc::new(Mutex::new(0));
    let incr_cycle = Arc::clone(&bcast_cycle);

    // error handling must be done within the threads
    let mqtt_thread = thread::spawn(move || {
        if let Err(err) = mqtt_entry(msg_tx, exit_cond, bcast_cycle) {
            error!("{}", err);
        }
    });
    let synch_thread = thread::spawn(move || {
        if let Err(err) = synch_entry(msg_rx, exit_cond2, incr_cycle) {
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
        todo!();
    } else {
        error!("malformed mqtt message");
    }
}

//? This command will not spawn new instances
//? if mosquitto already active.
pub fn start_mosquitto() -> Outcome<()> {
    debug!("server:start >> mosquitto daemon");
    if let Err(spawn_error) = process::Command::new("mosquitto").arg("-d").spawn() {
        return err_msg(format!(
            "Is mosquitto installed and in path? >> {}",
            spawn_error
        ));
    }
    Ok(())
}

//? This thread is to ensure no lost messages from mqtt
fn mqtt_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    exit_cond: Arc<Mutex<bool>>,
    cycle: Arc<Mutex<i32>>
) -> Outcome<()> {
    let (mqtt_client, mqtt_rx) =
        ipc::MqttClient::new(Some("localhost"), &["sinkd/clients"], "sinkd/server")?;

    loop {
        if utils::exited(&exit_cond) {
            return err_msg("server>> synch thread exited, aborting mqtt thread");
        }
        match mqtt_rx.try_recv() {
            Ok(msg) => {
                // ! process mqtt messages
                // need to figure out state of server before synchronizing
                let mut payload = ipc::decode(msg.unwrap().payload())?;

                if mqtt_client.publish(&mut payload).is_err() {
                    unimplemented!()
                }
                synch_tx.send(payload).unwrap(); // value moves/consumed here
            }
            Err(err) => match err {
                crossbeam::channel::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    debug!("server>> mqtt loop...")
                }
                crossbeam::channel::TryRecvError::Disconnected => {
                    utils::fatal(&exit_cond);
                    return err_msg("server>> mqtt_rx channel disconnected");
                }
            },
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
) -> Outcome<()> {
    loop {
        if utils::exited(&exit_cond) {
            return err_msg("server>> mqtt_thread exited, aborting synch thread");
        }
        match synch_rx.recv() { // blocking call
            Err(e) => {
                error!("server:synch_entry hangup on reciever?: {}", e);
            }
            Ok(payload) => {
                // let mut num = cycle.lock().unwrap();
                // *num += 1;
                utils::rsync(&payload);
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

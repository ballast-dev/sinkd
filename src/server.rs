//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
use paho_mqtt as mqtt;
use std::{
    fs,
    path::Path,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crate::{config, ipc, outcome::Outcome, parameters::Parameters};

static FATAL_FLAG: AtomicBool = AtomicBool::new(false);

pub fn start(params: &Parameters) -> Outcome<()> {
    ipc::start_mosquitto()?;
    ipc::daemon(init, "server", params)
}

pub fn stop(params: &Parameters) -> Outcome<()> {
    ipc::end_process(params)?;
    Ok(())
}

pub fn restart(params: &Parameters) -> Outcome<()> {
    match stop(params) {
        Ok(()) => {
            start(params)?;
            Ok(())
        }
        Err(e) => bad!(e),
    }
}

fn create_srv_dir(debug: bool) -> Outcome<()> {
    let path = if debug {
        Path::new("/tmp/sinkd/srv")
    } else if cfg!(target_os = "windows") {
        Path::new("/Program Files/sinkd/srv")
    } else if cfg!(target_os = "macos") {
        // NOTE: macos has System Integrity Protection (SIP)
        // or APFS volume protections, which do not allow /srv to be writable
        Path::new("/opt/sinkd/srv")
    } else {
        Path::new("/srv/sinkd")
    };

    if !path.exists() {
        if !debug && !config::have_permissions() {
            return bad!("Need elevated permissions to create {}", path.display());
        }
        match fs::create_dir_all(path) {
            Ok(()) => Ok(()),
            Err(e) => bad!("Unable to create '{}'  {}", path.display(), e),
        }
    } else {
        Ok(())
    }
}

fn init(params: &Parameters) -> Outcome<()> {
    create_srv_dir(params.debug)?;

    let (synch_tx, synch_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
        mpsc::channel();

    let bcast_cycle = Arc::new(Mutex::new(0));
    let incr_cycle = Arc::clone(&bcast_cycle);

    let state = Arc::new(Mutex::new(ipc::Status::Ready));
    let state2 = Arc::clone(&state);

    let status_thread = thread::spawn(move || {
        if let Err(err) = status_entry(synch_tx, &FATAL_FLAG, bcast_cycle, state) {
            error!("{}", err);
        }
    });
    let synch_thread = thread::spawn(move || {
        if let Err(err) = synch_entry(synch_rx, &FATAL_FLAG, incr_cycle, state2) {
            error!("{}", err);
        }
    });

    if let Err(status_thread_err) = status_thread.join() {
        error!(
            "server:status_thread join error! >> {:?}",
            status_thread_err
        );
        process::exit(1);
    }
    if let Err(synch_thread_err) = synch_thread.join() {
        error!("server::synch_thread join error! >> {:?}", synch_thread_err);
        process::exit(1);
    }
    Ok(())
}

//fn dispatch(msg: &Option<mqtt::Message>) {
//    if let Some(msg) = msg {
//        let payload = std::str::from_utf8(msg.payload()).unwrap();
//        debug!("topic: {}\tpayload: {}", msg.topic(), payload);
//        todo!();
//    } else {
//        error!("malformed mqtt message");
//    }
//}

//? This thread is to ensure no lost messages from mqtt
#[allow(unused_variables, unreachable_code)]
fn status_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    fatal_flag: &AtomicBool,
    cycle: Arc<Mutex<i32>>,
    state: Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    let (mqtt_client, mqtt_rx): (ipc::MqttClient, ipc::Rx) =
        match ipc::MqttClient::new(Some("localhost"), &["sinkd/clients"], "sinkd/server") {
            Ok((client, rx)) => (client, rx),
            Err(e) => {
                fatal_flag.store(true, Ordering::SeqCst);
                return bad!("Unable to create mqtt client, {}", e);
            }
        };

    loop {
        if fatal_flag.load(Ordering::SeqCst) {
            return bad!("server>> synch thread exited, aborting mqtt thread");
        }

        let mut status_payload = ipc::Payload::new()?.status(ipc::Status::Ready);
        if let Err(e) = mqtt_client.publish(&mut status_payload) {
            debug!("server:status_entry>> couldn't publish status? '{}'", e);
        }

        // then recv queries
        // and queue up rsync calls
        match mqtt_rx.try_recv() {
            Ok(msg) => {
                // ! process mqtt messages
                // need to figure out state of server before synchronizing
                if let Ok(mut state) = state.lock() {
                    match *state {
                        ipc::Status::NotReady(reason) => match reason {
                            ipc::Reason::Sinking => todo!(),
                            ipc::Reason::Behind => todo!(),
                            ipc::Reason::Other => todo!(),
                        },
                        ipc::Status::Ready => {
                            // TODO
                            // 1. recieve msg from client
                            // 2. if in good state switch status to "synchronizing"
                            // 3. call rsync <client> <server> <opts>
                            // 4. once finished switch state to "ready"
                            // TODO
                            debug!("server:status_entry>> State::Ready {:?}", msg);

                            let this_cycle = match cycle.lock() {
                                Ok(l) => *l,
                                Err(_e) => {
                                    fatal_flag.load(Ordering::SeqCst);
                                    error!("cycle lock busted");
                                    -1
                                }
                            };

                            if this_cycle == -1 {
                                return bad!("server>> cycle lock busted");
                            }

                            //let payload = match ipc::decode(msg.unwrap().payload()) {
                            if let Some(msg) = msg {
                                match ipc::decode(msg.payload()) {
                                    Ok(p) => {
                                        debug!("server:status_entry>> able to decode 😄 '{}'", p)
                                    }
                                    Err(e) => {
                                        debug!(
                                            "server:status_entry>> unable to decode 😦>> '{}'",
                                            e
                                        )
                                    }
                                };
                            } else {
                                debug!("server:status_entry>> recv empty msg")
                            }

                            //if payload.cycle as i32 >= this_cycle {
                            //    *state = ipc::Status::NotReady(ipc::Reason::Sinking);
                            //    todo!("call rsync");
                            //    if mqtt_client.publish(&mut payload).is_err() {
                            //        unimplemented!()
                            //    }
                            //    synch_tx.send(payload).unwrap(); // value moves/consumed here
                            //}
                        }
                    }
                } else {
                    fatal_flag.load(Ordering::SeqCst);
                    error!("state lock busted");
                }
            }
            Err(err) => match err {
                crossbeam::channel::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    debug!("server>> mqtt loop...");
                }
                crossbeam::channel::TryRecvError::Disconnected => {
                    fatal_flag.store(true, Ordering::SeqCst);
                    return bad!("server>> mqtt_rx channel disconnected");
                }
            },
        }
    }
}

// The engine behind sinkd is rsync
// With mqtt messages that are relevant invoke this and mirror current client
// to this server
#[allow(unused_variables)]
fn synch_entry(
    synch_rx: mpsc::Receiver<ipc::Payload>,
    fatal_flag: &AtomicBool,
    cycle: Arc<Mutex<i32>>,
    state: Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    loop {
        if fatal_flag.load(Ordering::SeqCst) {
            return bad!("server>> mqtt_thread exited, aborting synch thread");
        }
        // blocking call
        match synch_rx.recv() {
            Err(e) => {
                error!("server:synch_rx>> {}", e);
            }
            Ok(payload) => {
                // let mut num = cycle.lock().unwrap();
                // *num += 1;
                if let Ok(state_mutex) = state.lock() {
                    match *state_mutex {
                        ipc::Status::Ready => ipc::rsync(&payload),
                        ipc::Status::NotReady(_) => {
                            info!("not ready... wait 2 secs");
                            std::thread::sleep(Duration::from_secs(2)); // should be config driven
                        }
                    }
                } else {
                    fatal_flag.store(true, Ordering::SeqCst);
                    return bad!("state mutex poisoned");
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[allow(dead_code)]
fn publish(mqtt_client: &mqtt::Client, msg: &str) {
    if let Err(e) = mqtt_client.publish(mqtt::Message::new("sinkd/status", msg, mqtt::QOS_0)) {
        error!("server:publish >> {}", e);
    }
}

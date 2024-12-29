//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
use paho_mqtt as mqtt;
use std::{
    fs,
    path::PathBuf,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crate::{config, ipc, outcome::Outcome, parameters::Parameters, shiplog};

//static SRV_PATH: &str = {
//    #[cfg(target_os = "windows")]
//    { "/Program Files/sinkd/srv" }
//    #[cfg(target_os = "macos")]
//    { "/opt/sinkd/srv" }
//    #[cfg(target_os = "linux")]
//    { "/srv/sinkd" }
//};

pub fn start(params: &Parameters) -> Outcome<()> {
    ipc::start_mosquitto()?;
    println!("logging to: {}", params.log_path.display());
    ipc::daemon(init, params)
}

pub fn stop(params: &Parameters) -> Outcome<()> {
    let terminal_topic = format!("sinkd/{}/terminate", config::get_hostname()?);
    if let Err(e) = std::process::Command::new("mosquitto_pub")
        .arg("-h")
        .arg("localhost") // server stays local
        .arg("-t")
        .arg(terminal_topic)
        .arg("-m")
        .arg("end")
        .output()
    {
        println!("{:#?}", e);
    }
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

fn get_srv_dir(debug: u8) -> PathBuf {
    if debug > 0 {
        PathBuf::from("/tmp/sinkd/srv")
    } else if cfg!(target_os = "windows") {
        PathBuf::from("/Program Files/sinkd/srv")
    } else if cfg!(target_os = "macos") {
        // NOTE: macos has System Integrity Protection (SIP)
        // or APFS volume protections, which do not allow /srv to be writable
        PathBuf::from("/opt/sinkd/srv")
    } else {
        PathBuf::from("/srv/sinkd")
    }
}

fn create_srv_dir(debug: u8, path: &PathBuf) -> Outcome<()> {
    if !path.exists() {
        if debug == 0 && !config::have_permissions() {
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

// Daemonized call, stdin/stdout/stderr are closed
fn init(params: &Parameters) -> Outcome<()> {
    let srv_dir = get_srv_dir(params.debug);
    create_srv_dir(params.debug, &srv_dir)?;

    let (synch_tx, synch_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
        mpsc::channel();

    let fatal = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&fatal))?;

    let mqtt_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        move || {
            if let Err(err) = mqtt_entry(synch_tx, fatal) {
                error!("{}", err);
            }
        }
    });

    let synch_thread = thread::spawn({
        move || {
            if let Err(err) = synch_entry(synch_rx, fatal, srv_dir) {
                error!("{}", err);
            }
        }
    });

    if let Err(mqtt_thread_err) = mqtt_thread.join() {
        error!("server:status_thread join error! >> {:?}", mqtt_thread_err);
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
#[allow(unused_variables)]
fn mqtt_entry(synch_tx: mpsc::Sender<ipc::Payload>, fatal: Arc<AtomicBool>) -> Outcome<()> {
    let terminal_topic = format!("sinkd/{}/terminate", config::get_hostname()?);
    let (mqtt_client, mqtt_rx): (ipc::MqttClient, ipc::Rx) = match ipc::MqttClient::new(
        Some("localhost"), // server always spawn on current machine
        &["sinkd/clients", &terminal_topic],
        "sinkd/server",
    ) {
        Ok((client, rx)) => (client, rx),
        Err(e) => {
            fatal.store(true, Ordering::Relaxed);
            return bad!("Unable to create mqtt client, {}", e);
        }
    };

    // TODO: pervious cycle should be read from file
    // WARN: this needs to be config driven
    let mut cycle: u32 = 0;
    let mut state = ipc::Status::Ready;

    loop {
        if fatal.load(Ordering::Relaxed) {
            mqtt_client.disconnect();
            info!("server:mqtt_entry>> aborting");
            return Ok(());
        }

        if let Err(e) = broadcast_status(&mqtt_client, &state) {
            error!("{e}");
        }

        match mqtt_rx.try_recv() {
            Ok(msg) => {
                if let Some(msg) = msg {
                    if msg.topic() == terminal_topic {
                        debug!("server:mqtt_entry>> received terminal_topic");
                        fatal.store(true, Ordering::Relaxed);
                        continue;
                    }
                    match ipc::decode(msg.payload()) {
                        Ok(p) => {
                            debug!("server:mqtt_entry>> ⛵ decoded ⛵");

                            // TODO
                            // 1. recieve msg from client
                            // 2. if in good state switch status to "synchronizing"
                            // 3. call rsync <client> <server> <opts>
                            // 4. once finished switch state to "ready"

                            if let Err(e) =
                                queue(&synch_tx, &mqtt_client, p, &mut cycle, &mut state)
                            {
                                error!("{e}");
                            }
                        }
                        Err(e) => {
                            debug!("server:mqtt_entry>> unable to decode 😦>> '{}'", e)
                        }
                    };
                } else {
                    debug!("server:mqtt_entry>> recv empty msg")
                }
                // need to figure out state of server before synchronizing
            }
            Err(err) => match err {
                // NOTE: mqtt uses crossbeam
                crossbeam::channel::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    debug!("server:mqtt_entry>> waiting...");
                }
                crossbeam::channel::TryRecvError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("server>> mqtt_rx channel disconnected");
                }
            },
        }
    }
}

// check state of server and queue up payload if ready
fn queue(
    synch_tx: &mpsc::Sender<ipc::Payload>,
    mqtt_client: &ipc::MqttClient,
    payload: ipc::Payload,
    cycle: &mut u32,
    state: &mut ipc::Status,
) -> Outcome<()> {
    match state {
        ipc::Status::NotReady(reason) => match reason {
            // TODO: respond with mqtt message to client?
            ipc::Reason::Busy => todo!(),
            ipc::Reason::Behind => todo!(),
            ipc::Reason::Other => todo!(),
        },
        ipc::Status::Ready => {
            if let Err(e) = synch_tx.send(payload) {
                bad!("server:process>> unable to send on synch_tx {}", e)
            } else {
                Ok(())
            }
            //if payload.cycle > *cycle {
            //    *state = ipc::Status::NotReady(ipc::Reason::Busy);
            //    let mut response = ipc::Payload::new()?.status(state);
            //
            //    if let Err(e) = mqtt_client.publish(&mut response) {
            //        error!("server:queue>> unable to publish response {e}");
            //        return bad!("server:queue>> unable to publish response {}", e);
            //    }
            //    Ok(())
            //} else if payload.cycle == *cycle {
            //    info!("server:queue>> same payload cycle? no-op");
            //    Ok(())
            //} else {
            //    debug!("queuing payload: {payload:#?}");
            //    *cycle += 1;
            //    if let Err(e) = synch_tx.send(payload) {
            //        bad!("server:process>> unable to send on synch_tx {}", e)
            //    } else {
            //        Ok(())
            //    }
            //}
        }
    }
}

// The engine behind sinkd is rsync
// With mqtt messages that are relevant invoke this and mirror current client
// to this server. This will handle queued messages one at a time.
#[allow(unused_variables)]
fn synch_entry(
    synch_rx: mpsc::Receiver<ipc::Payload>,
    fatal: Arc<AtomicBool>,
    srv_dir: PathBuf,
) -> Outcome<()> {
    loop {
        if fatal.load(Ordering::Relaxed) {
            info!("server:synch_entry>> aborting");
            return Ok(());
        }

        match synch_rx.try_recv() {
            Err(recv_err) => match recv_err {
                mpsc::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    debug!("server:synch_entry>> waiting...");
                }
                mpsc::TryRecvError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("server>> _rx channel disconnected");
                }
            },
            Ok(payload) => {
                let dest = PathBuf::from(format!("{}/", &srv_dir.display()));
                ipc::rsync(&payload.src_paths, &dest)
            }
        }
    }
}

fn broadcast_status(mqtt_client: &ipc::MqttClient, state: &ipc::Status) -> Outcome<()> {
    let mut status_payload = ipc::Payload::new()?
        .dest_path("sinkd_status")
        .status(&state);

    if let Err(e) = mqtt_client.publish(&mut status_payload) {
        bad!("server:broadcast_status>> couldn't publish status? '{}'", e)
    } else {
        Ok(())
    }
}

//fn check_time(srv_state: &ipc::Status, payload: &ipc::Payload) {
//
//}

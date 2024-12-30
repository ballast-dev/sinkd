//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
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

use crate::{config, ipc, outcome::Outcome, parameters::Parameters};

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
    thread::sleep(Duration::from_millis(500));
    println!("logging to: {}", params.log_path.display());
    ipc::daemon(init, params)
}

pub fn stop() -> Outcome<()> {
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
    match stop() {
        Ok(()) => start(params),
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
    // TODO: this needs to be read from file, status + cycle number
    let status = Arc::new(Mutex::new(ipc::Status::Ready));

    let mqtt_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        let status = Arc::clone(&status);
        move || {
            if let Err(err) = mqtt_entry(synch_tx, fatal, status) {
                error!("{}", err);
            }
        }
    });

    let synch_thread = thread::spawn({
        move || {
            if let Err(err) = synch_entry(synch_rx, fatal, status, srv_dir) {
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

#[allow(unused_variables)]
fn mqtt_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    fatal: Arc<AtomicBool>,
    status: Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
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

    loop {
        if fatal.load(Ordering::Relaxed) {
            mqtt_client.disconnect();
            info!("server:mqtt_entry>> aborting");
            return Ok(());
        }

        if let Err(e) = broadcast_status(&mqtt_client, &status) {
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

                            if let Err(e) = queue(&synch_tx, &mqtt_client, p, &mut cycle, &status) {
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
    status: &Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    match status.lock() {
        Ok(state) => {
            if *state == ipc::Status::Ready {
                match payload.cycle.cmp(cycle) {
                    std::cmp::Ordering::Greater => {
                        debug!("queuing payload: {payload:#?}");
                        *cycle += 1;
                        if let Err(e) = synch_tx.send(payload) {
                            bad!("server:process>> unable to send on synch_tx {}", e)
                        } else {
                            Ok(())
                        }
                    }
                    std::cmp::Ordering::Equal => {
                        info!("server:queue>> same payload cycle? no-op");
                        Ok(())
                    }
                    std::cmp::Ordering::Less => {
                        let mut response =
                            ipc::Payload::new()?.status(&ipc::Status::NotReady(ipc::Reason::Busy));
                        if let Err(e) = mqtt_client.publish(&mut response) {
                            error!("server:queue>> unable to publish response {e}");
                        }
                        Ok(())
                    }
                }
            } else {
                // TODO: if the client is behind this repsonse tells the client to update
                let mut response = ipc::Payload::new()?.status(&state);
                if let Err(e) = mqtt_client.publish(&mut response) {
                    error!("server:queue>> unable to publish response {e}");
                }
                Ok(())
            }
        }
        Err(e) => bad!("server:queue>> status lock poisoned {}", e),
    }
}

// The engine behind sinkd is rsync
// With mqtt messages that are relevant invoke this and mirror current client
// to this server. This will handle queued messages one at a time.
#[allow(unused_variables)]
fn synch_entry(
    synch_rx: mpsc::Receiver<ipc::Payload>,
    fatal: Arc<AtomicBool>,
    status: Arc<Mutex<ipc::Status>>,
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
                // set busy let of lock as soon as possible
                let dest = PathBuf::from(format!("{}/", &srv_dir.display()));

                if let Ok(mut state) = status.lock() {
                    *state = ipc::Status::NotReady(ipc::Reason::Busy);
                } else {
                    error!("server:synch_entry>> unable to acquire status lock");
                    continue; // FIXME: fatal?
                }
                // this call could take a while
                ipc::rsync(&payload.src_paths, &dest);

                if let Ok(mut state) = status.lock() {
                    *state = ipc::Status::Ready;
                } else {
                    error!("server:synch_entry>> unable to acquire status lock");
                    continue;
                }
            }
        }
    }
}

fn broadcast_status(
    mqtt_client: &ipc::MqttClient,
    status: &Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    if let Ok(state) = status.lock() {
        let mut status_payload = ipc::Payload::new()?
            .dest_path("sinkd_status")
            .status(&state);
        if let Err(e) = mqtt_client.publish(&mut status_payload) {
            bad!("server:broadcast_status>> couldn't publish status? '{}'", e)
        } else {
            Ok(())
        }
    } else {
        bad!("status lock poisoned")
    }
}

//fn check_time(srv_state: &ipc::Status, payload: &ipc::Payload) {
//
//}

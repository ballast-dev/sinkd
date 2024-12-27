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

struct Bearing {
    pub fatal: AtomicBool,
    // cycle numbers in additoin to timestamps
    // provide a more robust way to check for 'out of synch'
    pub cycle: Mutex<i32>,
    pub state: Mutex<ipc::Status>,
}

impl Bearing {
    pub fn new() -> Self {
        Self {
            fatal: AtomicBool::new(false),
            cycle: Mutex::new(0),
            state: Mutex::new(ipc::Status::Ready),
        }
    }
}

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
    let (srv_addr, _) = config::get(&params)?;
    //ipc::end_process(params)
    if let Err(e) = std::process::Command::new("mosquitto_pub")
        .arg("-h")
        .arg(srv_addr)
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
    shiplog::init(&params)?;
    let srv_dir = get_srv_dir(params.debug);
    create_srv_dir(params.debug, &srv_dir)?;

    let (synch_tx, synch_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
        mpsc::channel();

    let fatal = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&fatal))?;
    let cycle = Arc::new(Mutex::new(0));
    let state = Arc::new(Mutex::new(ipc::Status::Ready));

    let mqtt_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        let cycle = Arc::clone(&cycle);
        let state = Arc::clone(&state);
        move || {
            if let Err(err) = mqtt_entry(synch_tx, fatal, cycle, state) {
                error!("{}", err);
            }
        }
    });

    let synch_thread = thread::spawn({
        move || {
            if let Err(err) = synch_entry(synch_rx, fatal, cycle, state, srv_dir) {
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
fn mqtt_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    fatal: Arc<AtomicBool>,
    cycle: Arc<Mutex<i32>>,
    state: Arc<Mutex<ipc::Status>>,
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

    loop {
        if fatal.load(Ordering::Relaxed) {
            mqtt_client.disconnect();
            info!("server:mqtt_entry>> aborting");
            return Ok(());
        }

        let mut status_payload = ipc::Payload::new()?
            .status(ipc::Status::Ready)
            .dest_path("sinkd_status");
        if let Err(e) = mqtt_client.publish(&mut status_payload) {
            debug!("server:mqtt_entry>> couldn't publish status? '{}'", e);
        }

        // then recv queries
        // and queue up rsync calls
        match mqtt_rx.try_recv() {
            Ok(msg) => {
                // need to figure out state of server before synchronizing
                if let Ok(state) = state.lock() {
                    match *state {
                        ipc::Status::NotReady(reason) => match reason {
                            ipc::Reason::Busy => todo!(),
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
                            debug!("server:mqtt_entry>> State::Ready {:?}", msg);

                            //let this_cycle = match cycle.lock() {
                            //    Ok(l) => *l,
                            //    Err(_e) => {
                            //        fatal_flag.load(Ordering::Relaxed);
                            //        error!("cycle lock busted");
                            //        return bad!("server>> cycle lock busted");
                            //    }
                            //};

                            //let payload = match ipc::decode(msg.unwrap().payload()) {
                            if let Some(msg) = msg {
                                match ipc::decode(msg.payload()) {
                                    Ok(p) => {
                                        debug!("server:mqtt_entry>> ⛵ decoded ⛵");
                                        if let Err(e) = synch_tx.send(p) {
                                            error!("server:synch_entry>> '{}'", e);
                                        }
                                    }
                                    Err(e) => {
                                        debug!("server:mqtt_entry>> unable to decode 😦>> '{}'", e)
                                    }
                                };
                            } else {
                                debug!("server:mqtt_entry>> recv empty msg")
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
                    fatal.store(true, Ordering::Relaxed);
                    error!("state lock busted");
                }
            }
            Err(err) => match err {
                crossbeam::channel::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    debug!("server>> mqtt loop...");
                }
                crossbeam::channel::TryRecvError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
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
    fatal: Arc<AtomicBool>,
    cycle: Arc<Mutex<i32>>,
    state: Arc<Mutex<ipc::Status>>,
    srv_dir: PathBuf,
) -> Outcome<()> {
    loop {
        if fatal.load(Ordering::Relaxed) {
            info!("server:sync_entry>> aborting");
            return Ok(());
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
                        ipc::Status::Ready => {
                            let dest = PathBuf::from(format!("{}/", &srv_dir.display()));
                            ipc::rsync(&payload.src_paths, &dest)
                        }
                        ipc::Status::NotReady(_) => {
                            info!("not ready... wait 2 secs");
                            std::thread::sleep(Duration::from_secs(2)); // should be config driven
                        }
                    }
                } else {
                    fatal.store(true, Ordering::Relaxed);
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

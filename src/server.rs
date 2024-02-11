//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
use crate::{
    fancy_debug, ipc,
    outcome::Outcome,
    shiplog,
    utils::{self, Parameters},
};
use mqtt::Message;
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
};

static FATAL_FLAG: AtomicBool = AtomicBool::new(false);

pub fn start(params: &Parameters) -> Outcome<()> {
    shiplog::init(params)?;
    utils::start_mosquitto()?;
    utils::daemon(init, "server", params)
}

pub fn stop(params: &Parameters) -> Outcome<()> {
    utils::end_process(params)?;
    Ok(())
}

pub fn restart(params: &Parameters) -> Outcome<()> {
    match stop(params) {
        Ok(_) => {
            start(params)?;
            Ok(())
        }
        Err(e) => return bad!(e),
    }
}

fn create_srv_dir(debug: bool) -> Outcome<()> {
    let path = if debug {
        Path::new("/tmp/sinkd/srv")
    } else {
        Path::new("/srv/sinkd")
    };

    if !path.exists() {
        if !debug && !utils::have_permissions() {
            return bad!("Need elevated permissions to create /srv/sinkd/");
        }
        match fs::create_dir_all(path) {
            Ok(_) => Ok(()),
            Err(e) => bad!("Unable to create '{}'  {}", path.display(), e),
        }
    } else {
        Ok(())
    }
}

fn init(params: &Parameters) -> Outcome<()> {
    // TODO: server path is `/srv/sinkd/<user>/...`
    create_srv_dir(params.debug)?;

    let (mqtt_tx, mqtt_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
        mpsc::channel();

    let bcast_cycle = Arc::new(Mutex::new(0));
    let incr_cycle = Arc::clone(&bcast_cycle);

    let state = Arc::new(Mutex::new(ipc::Status::Ready));
    let state2 = Arc::clone(&state);

    let status_thread = thread::spawn(move || {
        if let Err(err) = status_entry(mqtt_tx, &FATAL_FLAG, bcast_cycle, state) {
            error!("{}", err);
        }
    });
    let synch_thread = thread::spawn(move || {
        if let Err(err) = synch_entry(mqtt_rx, &FATAL_FLAG, incr_cycle, state2) {
            error!("{}", err);
        }
    });

    if let Err(status_thread_err) = status_thread.join() {
        error!("server:mqtt_thread join error! >> {:?}", status_thread_err);
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

//? This thread is to ensure no lost messages from mqtt
fn status_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    fatal_flag: &AtomicBool,
    cycle: Arc<Mutex<i32>>,
    state: Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    let (mqtt_client, mqtt_rx) =
        ipc::MqttClient::new(Some("localhost"), &["sinkd/clients"], "sinkd/server")?;

    loop {
        if fatal_flag.load(Ordering::SeqCst) {
            return bad!("server>> synch thread exited, aborting mqtt thread");
        }
        
        {// send status 
            let mut status_payload = ipc::Payload::new().status(ipc::Status::Ready);
            // let status_msg = ipc::encode(&mut status_payload)?;
            mqtt_client.publish(&mut status_payload)?;
        }
        // then recv queries
            // and queue up rsync calls
        match mqtt_rx.try_recv() {
            Ok(msg) => {
                // ! process mqtt messages
                // need to figure out state of server before synchronizing
                match state.lock() {
                    Ok(state) => match *state {
                        ipc::Status::NotReady(reason) => match reason {
                            ipc::Reason::Sinking => todo!(),
                            ipc::Reason::Behind => todo!(),
                            ipc::Reason::Other => todo!(),
                        },
                        ipc::Status::Ready => {
                            info!("recv: {:?}", msg);
                        }
                    },
                    Err(e) => {
                        fatal_flag.load(Ordering::SeqCst);
                        error!("state lock busted")
                    }
                }
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
                error!("server:synch_entry hangup on reciever?: {}", e);
            }
            Ok(payload) => {
                // let mut num = cycle.lock().unwrap();
                // *num += 1;
                if let Ok(state_mutex) = state.lock() {
                    match *state_mutex {
                        ipc::Status::Ready => utils::rsync(&payload),
                        ipc::Status::NotReady(_) => todo!()
                    }
                } else {
                    fatal_flag.store(true, Ordering::SeqCst);
                    return bad!("state mutex poisoned")
                }
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

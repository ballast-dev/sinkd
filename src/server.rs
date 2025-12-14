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
};

use crate::{config, ipc, outcome::Outcome, parameters::Parameters, rsync::rsync};

//static SRV_PATH: &str = {
//    #[cfg(target_os = "windows")]
//    { "/Program Files/sinkd/srv" }
//    #[cfg(target_os = "macos")]
//    { "/opt/sinkd/srv" }
//    #[cfg(target_os = "linux")]
//    { "/srv/sinkd" }
//};

pub fn start(params: &Parameters) -> Outcome<()> {
    // No need to start mosquitto - DDS is peer-to-peer
    println!("logging to: {}", params.log_path.display());
    ipc::daemon(init, params)
}

pub fn stop() -> Outcome<()> {
    let terminal_topic = format!("sinkd/{}/terminate", config::get_hostname()?);

    // Create a temporary DDS client just to send the terminate message
    match ipc::DdsClient::new(&[], &terminal_topic) {
        Ok((client, _rx)) => {
            let mut payload = ipc::Payload::new()?;
            payload.status = ipc::Status::NotReady(ipc::Reason::Other);
            if let Err(e) = client.publish(&mut payload) {
                println!("Failed to send terminate message: {e}");
            }
            client.disconnect();
        }
        Err(e) => {
            println!("Failed to create DDS client for termination: {e}");
        }
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
    if path.exists() {
        Ok(())
    } else {
        if debug == 0 && !config::have_permissions() {
            return bad!("Need elevated permissions to create {}", path.display());
        }
        match fs::create_dir_all(path) {
            Ok(()) => Ok(()),
            Err(e) => bad!("Unable to create '{}'  {}", path.display(), e),
        }
    }
}

// Daemonized call, stdin/stdout/stderr are closed
pub fn init(params: &Parameters) -> Outcome<()> {
    let srv_dir = get_srv_dir(params.debug);
    create_srv_dir(params.debug, &srv_dir)?;

    let (synch_tx, synch_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
        mpsc::channel();

    let fatal = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&fatal))?;
    // TODO: this needs to be read from file, status + cycle number
    let status = Arc::new(Mutex::new(ipc::Status::Ready));

    let dds_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        let status = Arc::clone(&status);
        move || {
            if let Err(err) = dds_entry(synch_tx, fatal, status) {
                error!("{err}");
            }
        }
    });

    let synch_thread = thread::spawn({
        move || {
            if let Err(err) = synch_entry(synch_rx, fatal, status, srv_dir) {
                error!("{err}");
            }
        }
    });

    if let Err(dds_thread_err) = dds_thread.join() {
        error!("server:dds_thread join error! >> {dds_thread_err:?}");
        process::exit(1);
    }
    if let Err(synch_thread_err) = synch_thread.join() {
        error!("server::synch_thread join error! >> {synch_thread_err:?}");
        process::exit(1);
    }
    Ok(())
}

#[allow(unused_variables)]
#[allow(clippy::needless_pass_by_value)]
fn dds_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    fatal: Arc<AtomicBool>,
    status: Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    let terminal_topic = format!("sinkd/{}/terminate", config::get_hostname()?);
    let (dds_client, dds_rx): (ipc::DdsClient, ipc::Rx) = match ipc::DdsClient::new(
        &[ipc::TOPIC_CLIENTS, &terminal_topic],
        ipc::TOPIC_SERVER,
    ) {
        Ok((client, rx)) => (client, rx),
        Err(e) => {
            fatal.store(true, Ordering::Relaxed);
            return bad!("Unable to create DDS client, {}", e);
        }
    };

    // TODO: pervious cycle should be read from file
    // WARN: this needs to be config driven
    let mut cycle: u32 = 0;

    loop {
        if fatal.load(Ordering::Relaxed) {
            dds_client.disconnect();
            info!("server:dds_entry>> aborting");
            return Ok(());
        }

        if let Err(e) = broadcast_status(&dds_client, &status) {
            error!("{e}");
        }

        match dds_rx.try_recv() {
            Ok(msg) => {
                if let Some(msg) = msg {
                    if msg.topic == terminal_topic {
                        debug!("server:dds_entry>> received terminal_topic");
                        fatal.store(true, Ordering::Relaxed);
                    } else {
                        debug!("server:dds_entry>> ⛵ received payload ⛵");

                        // TODO
                        // 1. recieve msg from client
                        // 2. if in good state switch status to "synchronizing"
                        // 3. call rsync <client> <server> <opts>
                        // 4. once finished switch state to "ready"

                        if let Err(e) =
                            queue(&synch_tx, &dds_client, msg.payload, &mut cycle, &status)
                        {
                            error!("{e}");
                        }
                    }
                } else {
                    debug!("server:dds_entry>> recv empty msg");
                }
            }
            Err(err) => match err {
                mpsc::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    debug!("server:dds_entry>> waiting...");
                }
                mpsc::TryRecvError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("server>> dds_rx channel disconnected");
                }
            },
        }
    }
}

// check state of server and queue up payload if ready
fn queue(
    synch_tx: &mpsc::Sender<ipc::Payload>,
    dds_client: &ipc::DdsClient,
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
                            ipc::Payload::new()?.status(ipc::Status::NotReady(ipc::Reason::Busy));
                        if let Err(e) = dds_client.publish(&mut response) {
                            error!("server:queue>> unable to publish response {e}");
                        }
                        Ok(())
                    }
                }
            } else {
                // TODO: if the client is behind this repsonse tells the client to update
                let mut response = ipc::Payload::new()?.status(*state);
                if let Err(e) = dds_client.publish(&mut response) {
                    error!("server:queue>> unable to publish response {e}");
                }
                Ok(())
            }
        }
        Err(e) => bad!("server:queue>> status lock poisoned {}", e),
    }
}

// The engine behind sinkd is rsync
// With DDS messages that are relevant invoke this and mirror current client
// to this server. This will handle queued messages one at a time.
#[allow(unused_variables)]
#[allow(clippy::needless_pass_by_value)]
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
                    // FIXME: fatal?
                }
                // this call could take a while
                rsync(&payload.src_paths, &dest);

                if let Ok(mut state) = status.lock() {
                    *state = ipc::Status::Ready;
                } else {
                    error!("server:synch_entry>> unable to acquire status lock");
                }
            }
        }
    }
}

fn broadcast_status(
    dds_client: &ipc::DdsClient,
    status: &Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    if let Ok(state) = status.lock() {
        let mut status_payload = ipc::Payload::new()?
            .dest_path("sinkd_status")
            .status(*state);
        if let Err(e) = dds_client.publish(&mut status_payload) {
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

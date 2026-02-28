//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
};

use serde::{Deserialize, Serialize};

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
    // No need to start mosquitto - Zenoh is peer-to-peer
    println!("logging to: {}", params.log_path.display());
    ipc::daemon(init, params)
}

pub fn stop() -> Outcome<()> {
    ipc::send_terminate_signal()
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

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedCycles {
    cycle_by_host: HashMap<String, u32>,
}

fn load_cycle_state(cycle_state_path: &PathBuf) -> HashMap<String, u32> {
    match fs::read_to_string(cycle_state_path) {
        Ok(content) => match toml::from_str::<PersistedCycles>(&content) {
            Ok(state) => state.cycle_by_host,
            Err(e) => {
                warn!(
                    "server: unable to parse cycle state '{}': {}",
                    cycle_state_path.display(),
                    e
                );
                HashMap::new()
            }
        },
        Err(_) => HashMap::new(),
    }
}

fn persist_cycle_state(cycle_state_path: &PathBuf, cycle_by_host: &HashMap<String, u32>) -> Outcome<()> {
    let state = PersistedCycles {
        cycle_by_host: cycle_by_host.clone(),
    };
    let serialized = toml::to_string(&state)
        .map_err(|e| format!("unable to serialize cycle state: {e}"))?;
    fs::write(cycle_state_path, serialized)?;
    Ok(())
}

// Daemonized call, stdin/stdout/stderr are closed
pub fn init(params: &Parameters) -> Outcome<()> {
    let srv_dir = get_srv_dir(params.debug);
    create_srv_dir(params.debug, &srv_dir)?;
    let cycle_state_path = srv_dir.join("cycle_state.toml");

    let (synch_tx, synch_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
        mpsc::channel();

    let fatal = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&fatal))?;
    // TODO: this needs to be read from file, status + cycle number
    let status = Arc::new(Mutex::new(ipc::Status::Ready));

    let zenoh_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        let status = Arc::clone(&status);
        let cycle_state_path = cycle_state_path.clone();
        move || {
            if let Err(err) = zenoh_entry(synch_tx, fatal, status, cycle_state_path) {
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

    if let Err(zenoh_thread_err) = zenoh_thread.join() {
        error!("server:zenoh_thread join error! >> {zenoh_thread_err:?}");
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
fn zenoh_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    fatal: Arc<AtomicBool>,
    status: Arc<Mutex<ipc::Status>>,
    cycle_state_path: PathBuf,
) -> Outcome<()> {
    let (zenoh_client, zenoh_rx, terminal_topic): (ipc::ZenohClient, ipc::Rx, String) =
        match ipc::connect_with_terminate_topic(&[ipc::TOPIC_CLIENTS], ipc::TOPIC_SERVER) {
            Ok(conn) => conn,
            Err(e) => {
                fatal.store(true, Ordering::Relaxed);
                return bad!("Unable to create Zenoh client, {}", e);
            }
        };

    // Track last accepted cycle per client hostname.
    let mut cycle_by_host: HashMap<String, u32> = load_cycle_state(&cycle_state_path);

    loop {
        if fatal.load(Ordering::Relaxed) {
            zenoh_client.disconnect();
            info!("server:zenoh_entry>> aborting");
            return Ok(());
        }

        if let Err(e) = broadcast_status(&zenoh_client, &status) {
            error!("{e}");
        }

        match zenoh_rx.try_recv() {
            Ok(msg) => {
                if let Err(e) = handle_incoming_transport_message(
                    msg,
                    terminal_topic.as_str(),
                    &fatal,
                    &synch_tx,
                    &zenoh_client,
                    &mut cycle_by_host,
                    &cycle_state_path,
                    &status,
                ) {
                    error!("{e}");
                }
            }
            Err(err) => match err {
                mpsc::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    debug!("server:zenoh_entry>> waiting...");
                }
                mpsc::TryRecvError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("server>> zenoh_rx channel disconnected");
                }
            },
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_incoming_transport_message(
    message: Option<ipc::ZenohMessage>,
    terminal_topic: &str,
    fatal: &Arc<AtomicBool>,
    synch_tx: &mpsc::Sender<ipc::Payload>,
    zenoh_client: &ipc::ZenohClient,
    cycle_by_host: &mut HashMap<String, u32>,
    cycle_state_path: &PathBuf,
    status: &Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    let Some(msg) = message else {
        debug!("server:zenoh_entry>> recv empty msg");
        return Ok(());
    };

    if msg.topic == terminal_topic {
        debug!("server:zenoh_entry>> received terminal_topic");
        fatal.store(true, Ordering::Relaxed);
        return Ok(());
    }

    debug!("server:zenoh_entry>> ⛵ received payload ⛵");
    queue(
        synch_tx,
        zenoh_client,
        msg.payload,
        cycle_by_host,
        cycle_state_path,
        status,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueueDecision {
    Enqueue,
    Duplicate,
    Stale,
}

fn decide_cycle(incoming_cycle: u32, last_cycle_for_host: Option<u32>) -> QueueDecision {
    match last_cycle_for_host {
        None => QueueDecision::Enqueue,
        Some(last) if incoming_cycle > last => QueueDecision::Enqueue,
        Some(last) if incoming_cycle == last => QueueDecision::Duplicate,
        Some(_) => QueueDecision::Stale,
    }
}

// check state of server and queue up payload if ready
fn queue(
    synch_tx: &mpsc::Sender<ipc::Payload>,
    zenoh_client: &ipc::ZenohClient,
    payload: ipc::Payload,
    cycle_by_host: &mut HashMap<String, u32>,
    cycle_state_path: &PathBuf,
    status: &Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    match status.lock() {
        Ok(state) => {
            if *state == ipc::Status::Ready {
                let decision = decide_cycle(
                    payload.cycle,
                    cycle_by_host.get(&payload.hostname).copied(),
                );
                match decision {
                    QueueDecision::Enqueue => {
                        debug!("queuing payload: {payload:#?}");
                        cycle_by_host.insert(payload.hostname.clone(), payload.cycle);
                        if let Err(e) = persist_cycle_state(cycle_state_path, cycle_by_host) {
                            warn!(
                                "server: unable to persist cycle state '{}': {}",
                                cycle_state_path.display(),
                                e
                            );
                        }
                        if let Err(e) = synch_tx.send(payload) {
                            bad!("server:process>> unable to send on synch_tx {}", e)
                        } else {
                            Ok(())
                        }
                    }
                    QueueDecision::Duplicate => {
                        info!("server:queue>> same payload cycle? no-op");
                        Ok(())
                    }
                    QueueDecision::Stale => {
                        let mut response =
                            ipc::Payload::new()?.status(ipc::Status::NotReady(ipc::Reason::Busy));
                        if let Err(e) = zenoh_client.publish(&mut response) {
                            error!("server:queue>> unable to publish response {e}");
                        }
                        Ok(())
                    }
                }
            } else {
                // TODO: if the client is behind this repsonse tells the client to update
                let mut response = ipc::Payload::new()?.status(*state);
                if let Err(e) = zenoh_client.publish(&mut response) {
                    error!("server:queue>> unable to publish response {e}");
                }
                Ok(())
            }
        }
        Err(e) => bad!("server:queue>> status lock poisoned {}", e),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};

    use super::{decide_cycle, load_cycle_state, persist_cycle_state, QueueDecision};

    #[test]
    fn cycle_decision_accepts_first_message() {
        assert_eq!(decide_cycle(1, None), QueueDecision::Enqueue);
    }

    #[test]
    fn cycle_decision_accepts_newer_message() {
        assert_eq!(decide_cycle(2, Some(1)), QueueDecision::Enqueue);
    }

    #[test]
    fn cycle_decision_ignores_duplicate_message() {
        assert_eq!(decide_cycle(7, Some(7)), QueueDecision::Duplicate);
    }

    #[test]
    fn cycle_decision_marks_older_message_as_stale() {
        assert_eq!(decide_cycle(3, Some(9)), QueueDecision::Stale);
    }

    #[test]
    fn cycle_state_roundtrip_persists_data() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sinkd_cycle_state_{unique}.toml"));

        let mut map = HashMap::new();
        map.insert("alpha".to_string(), 3);
        map.insert("bravo".to_string(), 9);

        persist_cycle_state(&path, &map).expect("persist should succeed");
        let loaded = load_cycle_state(&path);
        std::fs::remove_file(path).expect("temp file should be removable");

        assert_eq!(loaded.get("alpha"), Some(&3));
        assert_eq!(loaded.get("bravo"), Some(&9));
    }
}

// The engine behind sinkd is rsync
// With Zenoh messages that are relevant invoke this and mirror current client
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
                let rsync_cfg = payload.rsync.clone().unwrap_or_default();
                rsync(&payload.src_paths, &dest, &rsync_cfg);

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
    zenoh_client: &ipc::ZenohClient,
    status: &Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    if let Ok(state) = status.lock() {
        let mut status_payload = ipc::Payload::new()?
            .dest_path("sinkd_status")
            .status(*state);
        if let Err(e) = zenoh_client.publish(&mut status_payload) {
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

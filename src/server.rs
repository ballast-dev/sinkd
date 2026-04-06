//    ____
//   / __/__ _____  _____ ____
//  _\ \/ -_) __/ |/ / -_) __/
// /___/\__/_/  |___/\__/_/
use log::{debug, error, info, warn};

use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    config, ipc,
    outcome::Outcome,
    parameters::{DaemonParameters, ServerParameters},
    rsync::rsync,
};

const GENERATION_HISTORY_TTL_SECS: i64 = 7 * 24 * 3600;
const GENERATION_HISTORY_MAX: usize = 4096;

enum PostApply {
    Applied {
        writer_client_id: String,
        head_generation: u64,
    },
    StaleAtApply {
        head_generation: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HistoryEntry {
    generation: u64,
    saved_at_unix: i64,
}

#[derive(Debug, Default)]
struct GenerationState {
    current_generation: u64,
    history: Vec<HistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedGeneration {
    current_generation: u64,
    #[serde(default)]
    history: Vec<HistoryEntry>,
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

impl GenerationState {
    fn prune_history(&mut self, now_unix: i64) {
        self.history
            .retain(|e| now_unix - e.saved_at_unix <= GENERATION_HISTORY_TTL_SECS);
        while self.history.len() > GENERATION_HISTORY_MAX {
            self.history.remove(0);
        }
    }

    /// Returns the new head generation.
    fn bump(&mut self, now_unix: i64) -> u64 {
        self.current_generation = self.current_generation.saturating_add(1);
        let g = self.current_generation;
        self.history.push(HistoryEntry {
            generation: g,
            saved_at_unix: now_unix,
        });
        self.prune_history(now_unix);
        g
    }
}

fn load_generation_state(path: &Path) -> GenerationState {
    let Ok(content) = fs::read_to_string(path) else {
        return GenerationState::default();
    };
    let Ok(p) = toml::from_str::<PersistedGeneration>(&content) else {
        warn!(
            "server: unable to parse generation state '{}'",
            path.display()
        );
        return GenerationState::default();
    };
    let mut st = GenerationState {
        current_generation: p.current_generation,
        history: p.history,
    };
    st.prune_history(now_unix_secs());
    st
}

fn persist_generation_state(path: &Path, state: &GenerationState) -> Outcome<()> {
    let p = PersistedGeneration {
        current_generation: state.current_generation,
        history: state.history.clone(),
    };
    let serialized = toml::to_string(&p).map_err(|e| format!("serialize generation state: {e}"))?;
    fs::write(path, serialized)?;
    Ok(())
}

fn needs_push_basis_check(payload: &ipc::Payload) -> bool {
    matches!(payload.status, ipc::Status::Ready) && !payload.src_paths.is_empty()
}

pub fn start(params: &ServerParameters) -> Outcome<()> {
    // No need to start mosquitto - Zenoh is peer-to-peer
    println!("logging to: {}", params.shared.log_path.display());
    ipc::daemon(&DaemonParameters::Server(params.clone()))
}

pub fn stop() -> Outcome<()> {
    ipc::send_terminate_signal()
}

pub fn restart(params: &ServerParameters) -> Outcome<()> {
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

fn create_srv_dir(debug: u8, path: &Path) -> Outcome<()> {
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
pub fn init(params: &ServerParameters) -> Outcome<()> {
    let srv_dir = get_srv_dir(params.shared.debug);
    create_srv_dir(params.shared.debug, &srv_dir)?;
    let generation_state_path = srv_dir.join("generation_state.toml");

    let (synch_tx, synch_rx): (mpsc::Sender<ipc::Payload>, mpsc::Receiver<ipc::Payload>) =
        mpsc::channel();
    let (post_apply_tx, post_apply_rx) = mpsc::channel::<PostApply>();

    let fatal = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&fatal))?;
    let status = Arc::new(Mutex::new(ipc::Status::Ready));
    let generation_state = Arc::new(Mutex::new(load_generation_state(&generation_state_path)));

    let zenoh_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        let status = Arc::clone(&status);
        let generation_state = Arc::clone(&generation_state);
        let generation_state_path = generation_state_path.clone();
        move || {
            if let Err(err) = zenoh_entry(
                synch_tx,
                post_apply_rx,
                fatal,
                status,
                generation_state,
                generation_state_path,
            ) {
                error!("{err}");
            }
        }
    });

    let synch_thread = thread::spawn({
        let post_apply_tx = post_apply_tx.clone();
        let generation_state = Arc::clone(&generation_state);
        let generation_state_path = generation_state_path.clone();
        move || {
            if let Err(err) = synch_entry(
                synch_rx,
                fatal,
                status,
                srv_dir,
                post_apply_tx,
                generation_state,
                generation_state_path,
            ) {
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

fn publish_post_apply(zenoh_client: &ipc::ZenohClient, pa: PostApply) -> Outcome<()> {
    match pa {
        PostApply::Applied {
            writer_client_id,
            head_generation,
        } => {
            let mut p = ipc::Payload::new()?
                .dest_path("sinkd_status")
                .status(ipc::Status::Ready)
                .head_generation(head_generation)
                .last_writer_client_id(writer_client_id);
            zenoh_client.publish(&mut p)
        }
        PostApply::StaleAtApply { head_generation } => {
            let mut p = ipc::Payload::new()?
                .dest_path("sinkd_status")
                .status(ipc::Status::NotReady(ipc::Reason::Behind))
                .head_generation(head_generation);
            zenoh_client.publish(&mut p)
        }
    }
}

fn drain_post_apply(
    post_apply_rx: &mpsc::Receiver<PostApply>,
    zenoh_client: &ipc::ZenohClient,
) -> Outcome<()> {
    while let Ok(pa) = post_apply_rx.try_recv() {
        publish_post_apply(zenoh_client, pa)?;
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn zenoh_entry(
    synch_tx: mpsc::Sender<ipc::Payload>,
    post_apply_rx: mpsc::Receiver<PostApply>,
    fatal: Arc<AtomicBool>,
    status: Arc<Mutex<ipc::Status>>,
    generation_state: Arc<Mutex<GenerationState>>,
    generation_state_path: PathBuf,
) -> Outcome<()> {
    let (zenoh_client, zenoh_rx, terminal_topic): (ipc::ZenohClient, ipc::Rx, String) =
        match ipc::connect_with_terminate_topic(&[ipc::TOPIC_CLIENTS], ipc::TOPIC_SERVER) {
            Ok(conn) => conn,
            Err(e) => {
                fatal.store(true, Ordering::Relaxed);
                return bad!("Unable to create Zenoh client, {}", e);
            }
        };

    loop {
        if fatal.load(Ordering::Relaxed) {
            zenoh_client.disconnect();
            info!("server:zenoh_entry>> aborting");
            return Ok(());
        }

        if let Err(e) = drain_post_apply(&post_apply_rx, &zenoh_client) {
            error!("{e}");
            fatal.store(true, Ordering::Relaxed);
            zenoh_client.disconnect();
            return Err(e);
        }

        if let Err(e) = broadcast_status(&zenoh_client, &status, &generation_state) {
            error!("{e}");
            fatal.store(true, Ordering::Relaxed);
            zenoh_client.disconnect();
            return Err(e);
        }

        match zenoh_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(msg) => {
                if let Err(e) = handle_incoming_transport_message(
                    msg,
                    terminal_topic.as_str(),
                    &fatal,
                    &synch_tx,
                    &zenoh_client,
                    &generation_state,
                    generation_state_path.as_path(),
                    &status,
                ) {
                    error!("{e}");
                }
            }
            Err(err) => match err {
                RecvTimeoutError::Timeout => {
                    debug!("server:zenoh_entry>> waiting...");
                }
                RecvTimeoutError::Disconnected => {
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
    generation_state: &Arc<Mutex<GenerationState>>,
    generation_state_path: &Path,
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

    if msg.topic == ipc::TOPIC_CONTROL_RELOAD {
        let loaded = load_generation_state(generation_state_path);
        match generation_state.lock() {
            Ok(mut g) => *g = loaded,
            Err(e) => {
                error!("server:reload>> generation_state lock poisoned: {e}");
                fatal.store(true, Ordering::Relaxed);
                return bad!("server:reload>> generation_state lock poisoned: {e}");
            }
        }
        info!(
            "server: reloaded generation state from {}",
            generation_state_path.display()
        );
        return Ok(());
    }

    debug!("server:zenoh_entry>> ⛵ received payload ⛵");
    match queue(
        synch_tx,
        zenoh_client,
        msg.payload,
        generation_state,
        status,
    ) {
        Ok(()) => Ok(()),
        Err(e) => {
            fatal.store(true, Ordering::Relaxed);
            Err(e)
        }
    }
}

// check state of server and queue up payload if ready
fn queue(
    synch_tx: &mpsc::Sender<ipc::Payload>,
    zenoh_client: &ipc::ZenohClient,
    payload: ipc::Payload,
    generation_state: &Arc<Mutex<GenerationState>>,
    status: &Arc<Mutex<ipc::Status>>,
) -> Outcome<()> {
    match status.lock() {
        Ok(state) => {
            if *state == ipc::Status::Ready {
                if needs_push_basis_check(&payload) {
                    if payload.client_id.is_empty() {
                        let head = generation_state
                            .lock()
                            .map_err(|e| format!("server:queue>> generation_state lock: {e}"))?
                            .current_generation;
                        let mut response = ipc::Payload::new()?
                            .dest_path("sinkd_status")
                            .status(ipc::Status::NotReady(ipc::Reason::Behind))
                            .head_generation(head);
                        if let Err(e) = zenoh_client.publish(&mut response) {
                            error!("server:queue>> unable to publish response {e}");
                        }
                        return Ok(());
                    }
                    let head = generation_state
                        .lock()
                        .map_err(|e| format!("server:queue>> generation_state lock: {e}"))?
                        .current_generation;
                    if payload.basis_generation != head {
                        let mut response = ipc::Payload::new()?
                            .dest_path("sinkd_status")
                            .status(ipc::Status::NotReady(ipc::Reason::Behind))
                            .head_generation(head);
                        if let Err(e) = zenoh_client.publish(&mut response) {
                            error!("server:queue>> unable to publish response {e}");
                        }
                        return Ok(());
                    }
                }
                debug!("queuing payload: {payload:#?}");
                if let Err(e) = synch_tx.send(payload) {
                    bad!("server:process>> unable to send on synch_tx {}", e)
                } else {
                    Ok(())
                }
            } else {
                let head = generation_state
                    .lock()
                    .map_err(|e| format!("server:queue>> generation_state lock: {e}"))?
                    .current_generation;
                let mut response = ipc::Payload::new()?
                    .dest_path("sinkd_status")
                    .status(*state)
                    .head_generation(head);
                if let Err(e) = zenoh_client.publish(&mut response) {
                    error!("server:queue>> unable to publish response {e}");
                }
                Ok(())
            }
        }
        Err(e) => bad!("server:queue>> status lock poisoned {}", e),
    }
}

// The engine behind sinkd is rsync — bump global generation only after successful apply.
#[allow(clippy::needless_pass_by_value)]
fn synch_entry(
    synch_rx: mpsc::Receiver<ipc::Payload>,
    fatal: Arc<AtomicBool>,
    status: Arc<Mutex<ipc::Status>>,
    srv_dir: PathBuf,
    post_apply_tx: mpsc::Sender<PostApply>,
    generation_state: Arc<Mutex<GenerationState>>,
    generation_state_path: PathBuf,
) -> Outcome<()> {
    loop {
        if fatal.load(Ordering::Relaxed) {
            info!("server:synch_entry>> aborting");
            return Ok(());
        }

        match synch_rx.recv_timeout(Duration::from_secs(1)) {
            Err(recv_err) => match recv_err {
                RecvTimeoutError::Timeout => {
                    debug!("server:synch_entry>> waiting...");
                }
                RecvTimeoutError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("server>> _rx channel disconnected");
                }
            },
            Ok(payload) => {
                if needs_push_basis_check(&payload) {
                    let ok_basis = match generation_state.lock() {
                        Ok(st) => payload.basis_generation == st.current_generation,
                        Err(e) => {
                            error!("server:synch_entry>> generation_state lock: {e}");
                            fatal.store(true, Ordering::Relaxed);
                            return bad!("server:synch_entry>> generation_state lock: {e}");
                        }
                    };
                    if !ok_basis {
                        match status.lock() {
                            Ok(mut state) => *state = ipc::Status::Ready,
                            Err(e) => {
                                error!("server:synch_entry>> status mutex poisoned: {e}");
                                fatal.store(true, Ordering::Relaxed);
                                return bad!("server:synch_entry>> status lock poisoned: {e}");
                            }
                        }
                        let head = generation_state
                            .lock()
                            .map_err(|e| {
                                format!("server:synch_entry>> generation_state lock: {e}")
                            })?
                            .current_generation;
                        let _ = post_apply_tx.send(PostApply::StaleAtApply {
                            head_generation: head,
                        });
                        continue;
                    }
                }

                let dest = PathBuf::from(format!("{}/", &srv_dir.display()));

                match status.lock() {
                    Ok(mut state) => *state = ipc::Status::NotReady(ipc::Reason::Busy),
                    Err(e) => {
                        error!("server:synch_entry>> status mutex poisoned before rsync: {e}");
                        fatal.store(true, Ordering::Relaxed);
                        return bad!("server:synch_entry>> status lock poisoned before rsync: {e}");
                    }
                }
                let rsync_cfg = payload.rsync.clone().unwrap_or_default();
                let rsync_ok = rsync(&payload.src_paths, &dest, &rsync_cfg, None).is_ok();
                if !rsync_ok {
                    error!("server:synch_entry>> rsync failed");
                }

                match status.lock() {
                    Ok(mut state) => *state = ipc::Status::Ready,
                    Err(e) => {
                        error!("server:synch_entry>> status mutex poisoned after rsync: {e}");
                        fatal.store(true, Ordering::Relaxed);
                        return bad!("server:synch_entry>> status lock poisoned after rsync: {e}");
                    }
                }

                if rsync_ok {
                    let writer = payload.client_id.clone();
                    let new_gen = match generation_state.lock() {
                        Ok(mut st) => {
                            let new_gen = st.bump(now_unix_secs());
                            if let Err(e) = persist_generation_state(&generation_state_path, &st) {
                                error!(
                                    "server: unable to persist generation state '{}': {}",
                                    generation_state_path.display(),
                                    e
                                );
                            }
                            new_gen
                        }
                        Err(e) => {
                            error!("server:synch_entry>> generation_state lock: {e}");
                            fatal.store(true, Ordering::Relaxed);
                            return bad!("server:synch_entry>> generation_state lock: {e}");
                        }
                    };
                    let _ = post_apply_tx.send(PostApply::Applied {
                        writer_client_id: writer,
                        head_generation: new_gen,
                    });
                }
            }
        }
    }
}

fn broadcast_status(
    zenoh_client: &ipc::ZenohClient,
    status: &Arc<Mutex<ipc::Status>>,
    generation_state: &Arc<Mutex<GenerationState>>,
) -> Outcome<()> {
    let Ok(state) = status.lock() else {
        return bad!("status lock poisoned");
    };
    let Ok(gen) = generation_state.lock() else {
        return bad!("generation_state lock poisoned");
    };
    let head = gen.current_generation;
    let mut status_payload = ipc::Payload::new()?
        .dest_path("sinkd_status")
        .status(*state)
        .head_generation(head);
    if let Err(e) = zenoh_client.publish(&mut status_payload) {
        bad!("server:broadcast_status>> couldn't publish status? '{}'", e)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{load_generation_state, persist_generation_state, GenerationState};

    #[test]
    fn generation_state_roundtrip_persists_data() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sinkd_generation_state_{unique}.toml"));

        let mut st = GenerationState {
            current_generation: 4,
            ..Default::default()
        };
        let now = super::now_unix_secs();
        st.bump(now);

        persist_generation_state(&path, &st).expect("persist should succeed");
        let loaded = load_generation_state(&path);
        std::fs::remove_file(path).expect("temp file should be removable");

        assert_eq!(loaded.current_generation, 5);
        assert_eq!(loaded.history.len(), 1);
        assert_eq!(loaded.history[0].generation, 5);
    }
}

use clap::parser::ValuesRef;
use log::{debug, error, info, warn};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
        Arc, Mutex, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

use crate::{
    config::{self, SysConfig},
    ipc,
    outcome::Outcome,
    parameters::{ClientParameters, DaemonParameters},
    rsync::rsync,
};

struct ClientSyncState {
    client_id: String,
    acked_generation: u64,
    ack_path: PathBuf,
}

fn client_state_dir(params: &ClientParameters) -> PathBuf {
    let shared = &params.shared;
    if shared.debug > 0 {
        if let Some(p) = &params.client_state_dir_override {
            if !p.as_os_str().is_empty() {
                return p.clone();
            }
        }
        if let Ok(p) = std::env::var("SINKD_CLIENT_STATE_DIR") {
            let p = p.trim();
            if !p.is_empty() {
                return PathBuf::from(p);
            }
        }
        PathBuf::from("/tmp/sinkd/client")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\ProgramData\sinkd\client")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".local/share/sinkd")
    }
}

fn ensure_client_state_dir(params: &ClientParameters) -> Outcome<PathBuf> {
    let dir = client_state_dir(params);
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("client state dir '{}': {e}", dir.display()))?;
    }
    Ok(dir)
}

fn load_or_create_client_id(path: &Path) -> Outcome<String> {
    if let Ok(s) = fs::read_to_string(path) {
        let line = s.lines().next().unwrap_or("").trim();
        if !line.is_empty() {
            return Ok(line.to_string());
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("client_id parent '{}': {e}", parent.display()))?;
    }
    let id = uuid::Uuid::new_v4().to_string();
    fs::write(path, format!("{id}\n"))
        .map_err(|e| format!("write client_id: {e}"))?;
    Ok(id)
}

fn load_acked_generation(path: &Path) -> u64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn persist_acked_generation(path: &Path, acked: u64) -> Outcome<()> {
    fs::write(path, acked.to_string()).map_err(|e| format!("persist acked_generation: {e}"))?;
    Ok(())
}

fn load_client_sync_state(params: &ClientParameters) -> Outcome<Arc<Mutex<ClientSyncState>>> {
    let dir = ensure_client_state_dir(params)?;
    let id_path = dir.join("client_id");
    let ack_path = dir.join("acked_generation");
    let client_id = load_or_create_client_id(&id_path)?;
    let acked_generation = load_acked_generation(&ack_path);
    Ok(Arc::new(Mutex::new(ClientSyncState {
        client_id,
        acked_generation,
        ack_path,
    })))
}

fn attach_client_outbound_basis(
    payload: &mut ipc::Payload,
    sync: &Mutex<ClientSyncState>,
) -> Outcome<()> {
    let s = sync
        .lock()
        .map_err(|e| format!("client sync state lock: {e}"))?;
    payload.client_id.clear();
    payload.client_id.push_str(&s.client_id);
    payload.basis_generation = s.acked_generation;
    payload.head_generation = 0;
    payload.last_writer_client_id.clear();
    Ok(())
}

fn maybe_record_writer_ack(sync: &Mutex<ClientSyncState>, server_msg: &ipc::Payload) -> Outcome<()> {
    let mut s = sync
        .lock()
        .map_err(|e| format!("client sync state lock: {e}"))?;
    if server_msg.last_writer_client_id.is_empty() {
        return Ok(());
    }
    if server_msg.last_writer_client_id == s.client_id
        && server_msg.head_generation > s.acked_generation
    {
        s.acked_generation = server_msg.head_generation;
        persist_acked_generation(&s.ack_path, s.acked_generation)?;
    }
    Ok(())
}

fn record_pull_acked(sync: &Mutex<ClientSyncState>, head_generation: u64) -> Outcome<()> {
    if head_generation == 0 {
        return Ok(());
    }
    let mut s = sync
        .lock()
        .map_err(|e| format!("client sync state lock: {e}"))?;
    if head_generation > s.acked_generation {
        s.acked_generation = head_generation;
        persist_acked_generation(&s.ack_path, s.acked_generation)?;
    }
    Ok(())
}

/// Linux/Android use inotify-style backends without extra polling; other platforms fall back to periodic poll.
fn notify_config_for_platform() -> notify::Config {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        notify::Config::default()
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        notify::Config::default().with_poll_interval(Duration::from_secs(1))
    }
}

pub fn start(params: &ClientParameters) -> Outcome<()> {
    println!("logging to: {}", params.shared.log_path.display());
    ipc::daemon(&DaemonParameters::Client(params.clone()))
}

pub fn stop(_params: &ClientParameters) -> Outcome<()> {
    ipc::send_terminate_signal()
}

pub fn restart(params: &ClientParameters) -> Outcome<()> {
    match stop(params) {
        Ok(()) => {
            start(params)?;
            Ok(())
        }
        Err(e) => bad!(e),
    }
}

fn notify_reload() {
    if let Err(e) = ipc::publish_config_reload_signal() {
        warn!("config updated but could not publish reload notification over Zenoh: {e}");
    }
}

/// Anchors / users / list / log — same entrypoints as `sinkd client …` (daemon: [`start`], [`stop`], [`restart`]).
pub fn add(
    params: &ClientParameters,
    share_paths: &[&String],
    user_paths: &[&String],
) -> Outcome<()> {
    if share_paths.is_empty() && user_paths.is_empty() {
        return bad!("add: supply at least one --share and/or PATH");
    }

    let sys_path = params.system_config.as_ref().as_path();

    if !share_paths.is_empty() {
        let mut sys = config::load_system_config_file(sys_path)?;
        let anchors = sys.anchors.get_or_insert_with(Vec::new);
        for p in share_paths {
            let resolved = config::resolve(p)?;
            if anchors.iter().any(|a| a.path == resolved) {
                continue;
            }
            anchors.push(config::Anchor::with_path(resolved));
        }
        config::save_system_config_file(sys_path, &sys)?;
        info!("updated system config {}", sys_path.display());
    }

    if !user_paths.is_empty() {
        if params.user_configs.is_empty() {
            return bad!(
                "no user config files resolved; use --usr-cfg or create ~/.config/sinkd/sinkd.conf"
            );
        }
        for user_path in params.user_configs.iter() {
            let mut usr = config::load_user_config_file(user_path.as_path())?;
            for p in user_paths {
                let resolved = config::resolve(p)?;
                if usr.anchors.iter().any(|a| a.path == resolved) {
                    continue;
                }
                usr.anchors.push(config::Anchor::with_path(resolved));
            }
            config::save_user_config_file(user_path.as_path(), &usr)?;
            info!("updated user config {}", user_path.display());
        }
    }

    notify_reload();
    Ok(())
}

pub fn rm(
    params: &ClientParameters,
    share_paths: &[&String],
    user_paths: &[&String],
) -> Outcome<()> {
    if share_paths.is_empty() && user_paths.is_empty() {
        return bad!("remove: supply at least one --share and/or PATH");
    }

    let sys_path = params.system_config.as_ref().as_path();

    if !share_paths.is_empty() {
        let mut sys = config::load_system_config_file(sys_path)?;
        if let Some(anchors) = sys.anchors.as_mut() {
            for p in share_paths {
                let resolved = config::resolve(p)?;
                anchors.retain(|a| a.path != resolved);
            }
        }
        config::save_system_config_file(sys_path, &sys)?;
        info!("updated system config {}", sys_path.display());
    }

    if !user_paths.is_empty() {
        if params.user_configs.is_empty() {
            return bad!(
                "no user config files resolved; use --usr-cfg or create ~/.config/sinkd/sinkd.conf"
            );
        }
        for user_path in params.user_configs.iter() {
            let mut usr = config::load_user_config_file(user_path.as_path())?;
            for p in user_paths {
                let resolved = config::resolve(p)?;
                usr.anchors.retain(|a| a.path != resolved);
            }
            config::save_user_config_file(user_path.as_path(), &usr)?;
            info!("updated user config {}", user_path.display());
        }
    }

    notify_reload();
    Ok(())
}

pub fn adduser(params: &ClientParameters, users: Option<ValuesRef<String>>) -> Outcome<()> {
    let Some(users) = users else {
        return bad!("no user(s) were given!");
    };
    let sys_path = params.system_config.as_ref().as_path();
    let mut sys: SysConfig = config::load_system_config_file(sys_path)?;
    for user in users {
        if !sys.users.iter().any(|u| u == user.as_str()) {
            sys.users.push(user.as_str().to_string());
        }
    }
    config::save_system_config_file(sys_path, &sys)?;
    info!("updated system config {}", sys_path.display());
    notify_reload();
    Ok(())
}

pub fn rmuser(params: &ClientParameters, users: Option<ValuesRef<String>>) -> Outcome<()> {
    let Some(users) = users else {
        return bad!("no user(s) were given!");
    };
    let sys_path = params.system_config.as_ref().as_path();
    let mut sys: SysConfig = config::load_system_config_file(sys_path)?;
    for user in users {
        sys.users.retain(|u| u != user.as_str());
    }
    config::save_system_config_file(sys_path, &sys)?;
    info!("updated system config {}", sys_path.display());
    notify_reload();
    Ok(())
}

pub fn ls(
    params: &ClientParameters,
    paths: Option<Vec<&String>>,
    list_server: bool,
) -> Outcome<()> {
    if list_server {
        println!(
            "listing server-side paths is not implemented; inspect the server sync root (e.g. /srv/sinkd on Linux)."
        );
        return Ok(());
    }

    let (_addr, inode_map) = config::get(params)?;
    let mut keys: Vec<_> = inode_map.keys().cloned().collect();
    keys.sort();

    if let Some(filter) = paths {
        if filter.is_empty() {
            return bad!("no paths were given!");
        }
        let resolved: Vec<PathBuf> = filter
            .iter()
            .map(|p| config::resolve(p))
            .collect::<Result<_, _>>()?;
        for k in keys {
            if resolved.iter().any(|root| k.starts_with(root)) {
                println!("{}", k.display());
            }
        }
    } else {
        for k in keys {
            println!("{}", k.display());
        }
    }
    Ok(())
}

pub fn log(params: &ClientParameters) -> Outcome<()> {
    let data = fs::read_to_string(&params.shared.log_path).map_err(|e| {
        format!(
            "couldn't read log file {}: {e}",
            params.shared.log_path.display()
        )
    })?;
    print!("{data}");
    Ok(())
}

// Daemonized call, stdin/stdout/stderr are closed
pub fn init(params: &ClientParameters) -> Outcome<()> {
    let client_sync = load_client_sync_state(params)?;
    let params = Arc::new(params.clone());
    // `_srv_addr`: RSYNC destination / server address from TOML — reserved until the wire protocol
    // needs it here (currently paths use `Payload.hostname` for remote rsync).
    let (_srv_addr, inode_map) = config::get(params.as_ref())?;

    let (notify_tx, notify_rx): (mpsc::Sender<notify::Event>, mpsc::Receiver<notify::Event>) =
        mpsc::channel();
    let (event_tx, event_rx): (mpsc::Sender<PathBuf>, mpsc::Receiver<PathBuf>) = mpsc::channel();

    let watchers: Arc<Mutex<Vec<RecommendedWatcher>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let initial = setup_watchers(&inode_map, notify_tx.clone())?;
        *watchers
            .lock()
            .map_err(|e| format!("watchers lock poisoned: {e}"))? = initial;
    }

    let fatal = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&fatal))?;

    let inodes = Arc::new(RwLock::new(inode_map));

    let watch_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        let inode_map = Arc::clone(&inodes);
        // watch_thread needs a mutable map to assign "last event" to inode
        move || watch_entry(inode_map, notify_rx, event_tx, fatal)
    });

    let zenoh_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        let inode_map = Arc::clone(&inodes);
        let params = Arc::clone(&params);
        let watchers = Arc::clone(&watchers);
        let notify_tx = notify_tx.clone();
        let client_sync = Arc::clone(&client_sync);
        move || zenoh_entry(inode_map, event_rx, fatal, params, watchers, notify_tx, client_sync)
    });

    match watch_thread.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!("{e}"),
        Err(join_err) => return bad!("client:watch_thread join error! >> {:?}", join_err),
    }
    match zenoh_thread.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!("{e}"),
        Err(join_err) => return bad!("client:zenoh_thread join error! >> {:?}", join_err),
    }
    Ok(())
}

// This will check the event path against the known paths passed at config time
// Only top level paths are sent to the synch thread if the watched directory has exceeded
// interval. In other words events are filtered against intervals (per inode) and added
// to the synch queue.
fn check_interval(
    event_path: &Path,
    inode_map: &Arc<RwLock<config::InodeMap>>,
    event_tx: &mpsc::Sender<PathBuf>,
) -> Outcome<()> {
    // need to dynamically lookup keys and compare path names
    debug!("checking interval, event:{}", event_path.display());
    if let Ok(mut inode_map) = inode_map.write() {
        for (inode_path, inode) in inode_map.iter_mut() {
            if event_path.starts_with(inode_path) {
                let now = Instant::now();
                let elapse = now.duration_since(inode.last_event);
                if elapse >= inode.interval {
                    debug!("EVENT>> elapse: {}", elapse.as_secs());
                    inode.last_event = now;
                    if let Err(e) = event_tx.send(inode_path.clone()) {
                        return bad!("unable to send event path to sync queue: {}", e);
                    }
                }
                break;
            }
        }
        Ok(())
    } else {
        bad!("Unable to acquire RwLock for inode_map")
    }
}

#[allow(clippy::needless_pass_by_value)]
fn watch_entry(
    inode_map: Arc<RwLock<config::InodeMap>>,
    notify_rx: mpsc::Receiver<notify::Event>,
    event_tx: mpsc::Sender<PathBuf>,
    fatal: Arc<AtomicBool>,
) -> Outcome<()> {
    loop {
        if fatal.load(Ordering::Relaxed) {
            info!("client:watch_entry>> aborting");
            return Ok(());
        }

        match notify_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(event) => {
                if matches!(
                    event.kind,
                    notify::EventKind::Create(_)
                        | notify::EventKind::Modify(_)
                        | notify::EventKind::Remove(_)
                ) {
                    for path in &event.paths {
                        check_interval(path, &inode_map, &event_tx)?;
                    }
                }
            }
            Err(err) => match err {
                RecvTimeoutError::Timeout => {}
                RecvTimeoutError::Disconnected => {
                    error!("FATAL: notify_rx hung up in watch_entry");
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("client:watch_entry>> notify_rx disconnected");
                }
            },
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn zenoh_entry(
    inode_map: Arc<RwLock<config::InodeMap>>,
    event_rx: mpsc::Receiver<PathBuf>,
    fatal: Arc<AtomicBool>,
    params: Arc<ClientParameters>,
    watchers: Arc<Mutex<Vec<RecommendedWatcher>>>,
    notify_tx: mpsc::Sender<Event>,
    client_sync: Arc<Mutex<ClientSyncState>>,
) -> Outcome<()> {
    let (zenoh_client, zenoh_rx, terminal_topic): (ipc::ZenohClient, ipc::Rx, String) =
        match ipc::connect_with_terminate_topic(&[ipc::TOPIC_SERVER], ipc::TOPIC_CLIENTS) {
            Ok(conn) => conn,
            Err(e) => {
                fatal.store(true, Ordering::Relaxed);
                return bad!("Unable to create Zenoh client, {}", e);
            }
        };

    // The server will send status updates to it's clients every 5 seconds
    loop {
        if fatal.load(Ordering::Relaxed) {
            zenoh_client.disconnect();
            info!("client:zenoh_entry>> aborting");
            return Ok(());
        }

        match zenoh_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(message) => {
                if let Err(e) = handle_incoming_transport_message(
                    message,
                    terminal_topic.as_str(),
                    &fatal,
                    &event_rx,
                    &zenoh_client,
                    &inode_map,
                    params.as_ref(),
                    &watchers,
                    &notify_tx,
                    &client_sync,
                ) {
                    error!("client:zenoh_entry>> process: {e}");
                }
            }
            Err(e) => match e {
                RecvTimeoutError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("client:zenoh_entry>> zenoh_rx hung up?");
                }
                RecvTimeoutError::Timeout => {
                    debug!("client:zenoh_entry>> waiting on message...");
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
    event_rx: &mpsc::Receiver<PathBuf>,
    zenoh_client: &ipc::ZenohClient,
    inode_map: &Arc<RwLock<config::InodeMap>>,
    params: &ClientParameters,
    watchers: &Arc<Mutex<Vec<RecommendedWatcher>>>,
    notify_tx: &mpsc::Sender<Event>,
    client_sync: &Arc<Mutex<ClientSyncState>>,
) -> Outcome<()> {
    let Some(msg) = message else {
        return bad!("client:zenoh_entry>> empty message?");
    };

    if msg.topic == terminal_topic {
        debug!("client:zenoh_entry>> received terminal_topic");
        fatal.store(true, Ordering::Relaxed);
        return Ok(());
    }

    if msg.topic == ipc::TOPIC_CONTROL_RELOAD {
        return apply_client_config_reload(params, inode_map, watchers, notify_tx);
    }

    // process Zenoh traffic from server
    debug!("client>> 👍 recv: {}", msg.payload);
    process(
        event_rx,
        zenoh_client,
        inode_map,
        client_sync,
        &msg.payload,
    )
}

fn process(
    event_rx: &mpsc::Receiver<PathBuf>,
    zenoh_client: &ipc::ZenohClient,
    inode_map: &Arc<RwLock<config::InodeMap>>,
    client_sync: &Arc<Mutex<ClientSyncState>>,
    server_msg: &ipc::Payload,
) -> Outcome<()> {
    maybe_record_writer_ack(client_sync, server_msg)?;

    match server_msg.status {
        ipc::Status::NotReady(reason) => match reason {
            ipc::Reason::Busy => {
                info!("client:process>> server busy... wait 5 secs");
                std::thread::sleep(Duration::from_secs(5)); // should be config driven
                Ok(())
            }
            ipc::Reason::Behind => {
                debug!("client:process>> Behind synch up");
                // let's sync up
                if let Ok(map_read) = inode_map.read() {
                    let head = server_msg.head_generation;
                    let src_paths = map_read.keys().cloned().collect();
                    let mut payload = ipc::Payload::new()?
                        .status(ipc::Status::NotReady(ipc::Reason::Behind))
                        .src_paths(src_paths);
                    pull(&payload)?;
                    attach_client_outbound_basis(&mut payload, client_sync)?;
                    zenoh_client.publish(&mut payload)?;
                    record_pull_acked(client_sync, head)?;
                    Ok(())
                } else {
                    bad!("unable to acquire inode_map read lock")
                }
            }

            ipc::Reason::Other => {
                warn!("client:process>> unhandled NotReady(Other); no action");
                Ok(())
            }
        },
        ipc::Status::Ready => {
            debug!("client:process>> ipc::Status::Ready");
            match filter_file_events(event_rx) {
                Ok(filtered_paths) => {
                    if filtered_paths.is_empty() {
                        debug!("client:process>> nothing to send");
                    } else {
                        let grouped_paths = if let Ok(map_read) = inode_map.read() {
                            let mut grouped: HashMap<config::ResolvedRsyncConfig, Vec<PathBuf>> =
                                HashMap::new();
                            for path in filtered_paths {
                                let rsync_cfg = map_read
                                    .get(&path)
                                    .map_or_else(config::ResolvedRsyncConfig::default, |inode| {
                                        inode.rsync.clone()
                                    });
                                grouped.entry(rsync_cfg).or_default().push(path);
                            }
                            grouped
                        } else {
                            return bad!("unable to acquire inode_map read lock");
                        };

                        for (rsync_cfg, paths) in grouped_paths {
                            let mut payload = ipc::Payload::new()?
                                .src_paths(paths)
                                .rsync(rsync_cfg);
                            attach_client_outbound_basis(&mut payload, client_sync)?;
                            if let Err(e) = zenoh_client.publish(&mut payload) {
                                error!("unable to publish {e}");
                            } else {
                                info!("published payload: {payload}");
                            }
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    bad!("unable to filter_paths: {}", e)
                }
            }
        }
    }
}

fn apply_client_config_reload(
    params: &ClientParameters,
    inode_map: &Arc<RwLock<config::InodeMap>>,
    watchers: &Arc<Mutex<Vec<RecommendedWatcher>>>,
    notify_tx: &mpsc::Sender<Event>,
) -> Outcome<()> {
    let (_srv_addr, new_map) = config::get(params)?;
    let new_watchers = setup_watchers(&new_map, notify_tx.clone())?;
    {
        let mut im = inode_map
            .write()
            .map_err(|e| format!("inode_map write lock poisoned: {e}"))?;
        *im = new_map;
    }
    {
        let mut w = watchers
            .lock()
            .map_err(|e| format!("watchers lock poisoned: {e}"))?;
        *w = new_watchers;
    }
    info!("client: configuration reloaded from disk");
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn setup_watchers(
    inode_map: &config::InodeMap,
    tx: mpsc::Sender<Event>,
) -> Outcome<Vec<RecommendedWatcher>> {
    let mut watchers: Vec<RecommendedWatcher> = Vec::new();

    for pathbuf in inode_map.keys() {
        // Clone tx for use in this iteration
        let tx_clone = tx.clone();

        // Create a watcher with initial configuration
        let notify_cfg = notify_config_for_platform();
        let mut watcher = RecommendedWatcher::new(
            move |res| match res {
                Ok(event) => {
                    if tx_clone.send(event).is_err() {
                        error!("failed to send notify event");
                    }
                }
                Err(err) => error!("watch error: {err:?}"),
            },
            notify_cfg,
        )
        .map_err(|e| format!("couldn't create watcher: {e}"))?;

        if watcher.watch(pathbuf, RecursiveMode::Recursive).is_err() {
            warn!("unable to set watcher for: '{}'", pathbuf.display());
        } else {
            info!("set watcher for: '{}'", pathbuf.display());
            watchers.push(watcher);
        }
    }

    if watchers.is_empty() {
        bad!("nothing to watch! aborting")
    } else {
        Ok(watchers)
    }
}

// Will loop on file events until queue (channel) is empty
// Using a HashSet to filter out redundancies will return
// sanitized list of paths ready to send to sinkd server
// TODO: need to account for serveral users
fn filter_file_events(event_rx: &mpsc::Receiver<PathBuf>) -> Outcome<Vec<PathBuf>> {
    let mut path_set = HashSet::<PathBuf>::new();
    loop {
        match event_rx.try_recv() {
            // only top level paths are passed
            Ok(path) => {
                debug!("Adding file to path_set");
                path_set.insert(path);
            }
            Err(err) => match err {
                mpsc::TryRecvError::Disconnected => return bad!("event_rx disconnected"),
                mpsc::TryRecvError::Empty => break, // Ready to send!
            },
        }
    }

    Ok(path_set.into_iter().collect())
}

#[allow(dead_code)]
fn push(payload: &ipc::Payload) -> Outcome<()> {
    let dest = PathBuf::from(format!(
        "{}:{}",
        payload.hostname,
        payload.dest_path.display()
    ));
    debug!(
        "pulling srcs:[{}] dest:{}",
        payload
            .src_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        dest.display()
    );
    let rsync_cfg = payload.rsync.clone().unwrap_or_default();
    rsync(&payload.src_paths, &dest, &rsync_cfg)
}

fn pull(payload: &ipc::Payload) -> Outcome<()> {
    let srcs: Vec<PathBuf> = payload
        .src_paths
        .iter()
        .map(|p| PathBuf::from(format!("{}:{}", payload.hostname, p.display())))
        .collect();

    debug!(
        "pulling srcs:[{}] dest:{}",
        srcs.iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        payload.dest_path.display()
    );

    let rsync_cfg = payload.rsync.clone().unwrap_or_default();
    rsync(&srcs, &payload.dest_path, &rsync_cfg)
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::mpsc};

    use super::filter_file_events;

    #[test]
    fn filter_file_events_deduplicates_paths() {
        let (tx, rx) = mpsc::channel();
        let path = PathBuf::from("/tmp/a");
        tx.send(path.clone()).expect("send should succeed");
        tx.send(path.clone()).expect("send should succeed");
        tx.send(PathBuf::from("/tmp/b"))
            .expect("send should succeed");

        let mut filtered = filter_file_events(&rx).expect("filter should succeed");
        filtered.sort();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0], PathBuf::from("/tmp/a"));
        assert_eq!(filtered[1], PathBuf::from("/tmp/b"));
    }

    #[test]
    fn filter_file_events_returns_error_when_channel_disconnected() {
        let (tx, rx) = mpsc::channel::<PathBuf>();
        drop(tx);

        let err = filter_file_events(&rx).expect_err("disconnect should return an error");
        assert_eq!(err.to_string(), "event_rx disconnected");
    }
}

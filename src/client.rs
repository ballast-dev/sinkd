use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

use crate::{bad, config, ipc, outcome::Outcome, parameters::Parameters, rsync::rsync};

pub fn start(params: &Parameters) -> Outcome<()> {
    println!("logging to: {}", params.log_path.display());
    ipc::daemon(init, params)
}

pub fn stop(params: &Parameters) -> Outcome<()> {
    let _ = config::get(params)?;
    ipc::send_terminate_signal()?;
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

// Daemonized call, stdin/stdout/stderr are closed
pub fn init(params: &Parameters) -> Outcome<()> {
    let (_srv_addr, inode_map) = config::get(params)?;

    let (notify_tx, notify_rx): (mpsc::Sender<notify::Event>, mpsc::Receiver<notify::Event>) =
        mpsc::channel();
    let (event_tx, event_rx): (mpsc::Sender<PathBuf>, mpsc::Receiver<PathBuf>) = mpsc::channel();

    // keep the watchers alive!
    let _watchers = match setup_watchers(&inode_map, notify_tx) {
        Ok(w) => w,
        Err(e) => return bad!("{}", e),
    };

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
        move || zenoh_entry(inode_map, event_rx, fatal)
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

        match notify_rx.try_recv() {
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
                mpsc::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                mpsc::TryRecvError::Disconnected => {
                    error!("FATAL: notify_rx hung up in watch_entry");
                    fatal.store(true, Ordering::Relaxed);
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
) -> Outcome<()> {
    let (zenoh_client, zenoh_rx, terminal_topic): (ipc::ZenohClient, ipc::Rx, String) =
        match ipc::connect_with_terminate_topic(&[ipc::TOPIC_SERVER], ipc::TOPIC_CLIENTS) {
            Ok(conn) => conn,
            Err(e) => {
                fatal.store(true, Ordering::Relaxed);
                return bad!("Unable to create Zenoh client, {}", e);
            }
        };

    //// assume we are behind
    //let mut server_status = ipc::Status::NotReady(ipc::Reason::Behind);
    let mut cycle: u32 = 0; // WARN: maybe this should be read from disk

    // The server will send status updates to it's clients every 5 seconds
    loop {
        if fatal.load(Ordering::Relaxed) {
            zenoh_client.disconnect();
            info!("client:zenoh_entry>> aborting");
            return Ok(());
        }

        match zenoh_rx.try_recv() {
            Ok(message) => {
                if let Err(e) = handle_incoming_transport_message(
                    message,
                    terminal_topic.as_str(),
                    &fatal,
                    &event_rx,
                    &zenoh_client,
                    &inode_map,
                    &mut cycle,
                ) {
                    error!("client:zenoh_entry>> process: {e}");
                }
            }
            Err(e) => match e {
                mpsc::TryRecvError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("client:zenoh_entry>> zenoh_rx hung up?");
                }
                mpsc::TryRecvError::Empty => {
                    debug!("client:zenoh_entry>> waiting on message...");
                }
            },
        }

        // TODO: add 'system_interval' to config
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn handle_incoming_transport_message(
    message: Option<ipc::ZenohMessage>,
    terminal_topic: &str,
    fatal: &Arc<AtomicBool>,
    event_rx: &mpsc::Receiver<PathBuf>,
    zenoh_client: &ipc::ZenohClient,
    inode_map: &Arc<RwLock<config::InodeMap>>,
    cycle: &mut u32,
) -> Outcome<()> {
    let Some(msg) = message else {
        return bad!("client:zenoh_entry>> empty message?");
    };

    if msg.topic == terminal_topic {
        debug!("client:zenoh_entry>> received terminal_topic");
        fatal.store(true, Ordering::Relaxed);
        return Ok(());
    }

    // process Zenoh traffic from server
    debug!("client>> 👍 recv: {}", msg.payload);
    process(event_rx, zenoh_client, inode_map, msg.payload.status, cycle)
}

fn process(
    event_rx: &mpsc::Receiver<PathBuf>,
    zenoh_client: &ipc::ZenohClient,
    inode_map: &Arc<RwLock<config::InodeMap>>,
    status: ipc::Status,
    cycle: &mut u32,
) -> Outcome<()> {
    match status {
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
                    let src_paths = map_read.keys().cloned().collect();
                    let mut payload = ipc::Payload::new()?
                        .status(ipc::Status::NotReady(ipc::Reason::Behind))
                        .src_paths(src_paths);
                    pull(&payload);
                    zenoh_client.publish(&mut payload)
                } else {
                    bad!("unable to acquire inode_map read lock")
                }
            }

            ipc::Reason::Other => todo!(),
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
                            *cycle += 1;
                            let mut payload = ipc::Payload::new()?
                                .src_paths(paths)
                                .cycle(*cycle)
                                .rsync(rsync_cfg);
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
        let mut watcher = RecommendedWatcher::new(
            move |res| match res {
                Ok(event) => {
                    if tx_clone.send(event).is_err() {
                        error!("failed to send notify event");
                    }
                }
                Err(err) => error!("watch error: {err:?}"),
            },
            notify::Config::default().with_poll_interval(std::time::Duration::from_secs(1)), // Set polling interval
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

    // buffered events
    let mut event_paths: Vec<PathBuf> = vec![];
    if !path_set.is_empty() {
        for path in path_set.drain() {
            event_paths.push(path);
        }
    }
    Ok(event_paths)
}

#[allow(dead_code)]
fn push(payload: &ipc::Payload) {
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
    rsync(&payload.src_paths, &dest, &rsync_cfg);
}

fn pull(payload: &ipc::Payload) {
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
    rsync(&srcs, &payload.dest_path, &rsync_cfg);
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

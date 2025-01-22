use crossbeam::channel::TryRecvError;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashSet,
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
    let terminal_topic = format!("sinkd/{}/terminate", config::get_hostname()?);
    let (srv_addr, _) = config::get(params)?;
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

// Daemonized call, stdin/stdout/stderr are closed
pub fn init(params: &Parameters) -> Outcome<()> {
    let (srv_addr, inode_map) = config::get(params)?;

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

    let mqtt_thread = thread::spawn({
        let fatal = Arc::clone(&fatal);
        let inode_map = Arc::clone(&inodes);
        move || mqtt_entry(&srv_addr, inode_map, event_rx, fatal)
    });

    if let Err(e) = watch_thread.join().unwrap() {
        error!("{}", e);
    }
    if let Err(e) = mqtt_thread.join().unwrap() {
        error!("{}", e);
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
                    event_tx.send(inode_path.clone()).unwrap(); // to kick off sync thread
                }
                break;
            }
        }
        Ok(())
    } else {
        bad!("Unable to acquire RwLock for inode_map")
    }
}

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

fn mqtt_entry(
    server_addr: &str,
    inode_map: Arc<RwLock<config::InodeMap>>,
    event_rx: mpsc::Receiver<PathBuf>,
    fatal: Arc<AtomicBool>,
) -> Outcome<()> {
    let _payload = ipc::Payload::new();

    let terminal_topic = format!("sinkd/{}/terminate", config::get_hostname()?);
    let (mqtt_client, mqtt_rx): (ipc::MqttClient, ipc::Rx) = match ipc::MqttClient::new(
        Some(server_addr),
        &["sinkd/server", &terminal_topic],
        "sinkd/clients",
    ) {
        Ok((client, rx)) => (client, rx),
        Err(e) => {
            fatal.store(true, Ordering::Relaxed);
            return bad!("Unable to create mqtt client, {}", e);
        }
    };

    //// assume we are behind
    //let mut server_status = ipc::Status::NotReady(ipc::Reason::Behind);
    let mut cycle: u32 = 0; // WARN: maybe this should be read from disk

    // The server will send status updates to it's clients every 5 seconds
    loop {
        if fatal.load(Ordering::Relaxed) {
            mqtt_client.disconnect();
            info!("client:mqtt_entry>> aborting");
            return Ok(());
        }

        match mqtt_rx.try_recv() {
            Ok(message) => {
                if let Some(msg) = message {
                    if msg.topic() == terminal_topic {
                        debug!("client:mqtt_entry>> received terminal_topic");
                        fatal.store(true, Ordering::Relaxed);
                    } else if let Ok(decoded_payload) = ipc::decode(msg.payload()) {
                        // process mqtt traffic from server
                        debug!("client>> 👍 recv: {}", decoded_payload);
                        if let Err(e) = process(
                            &event_rx,
                            &mqtt_client,
                            &inode_map,
                            decoded_payload.status,
                            &mut cycle,
                        ) {
                            error!("client:mqtt_entry>> process: {}", e);
                        }
                    } else {
                        error!(
                            "client:mqtt_entry>> unable to decode message: {:?}",
                            msg.payload()
                        );
                    }
                } else {
                    error!("client:mqtt_entry>> empty message?");
                }
            }
            Err(e) => match e {
                TryRecvError::Disconnected => {
                    fatal.store(true, Ordering::Relaxed);
                    return bad!("client:mqtt_entry>> mqtt_rx hung up?");
                }
                TryRecvError::Empty => {
                    debug!("client:mqtt_entry>> waiting on message...");
                }
            },
        }

        // test from client to server
        //match ipc::Payload::new() {
        //    Ok(mut p) => {
        //        p = p.src_paths(vec![PathBuf::from("debug/path")]);
        //        if let Err(e) = mqtt_client.publish(&mut p) {
        //            debug!("client:mqtt_entry>> can't publish?  {e}");
        //        }
        //    }
        //    Err(e) => debug!("client:mqtt_entry>> unable to create payload {e}"),
        //};

        // TODO: add 'system_interval' to config
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn process(
    event_rx: &mpsc::Receiver<PathBuf>,
    mqtt_client: &ipc::MqttClient,
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
                        .status(&ipc::Status::NotReady(ipc::Reason::Behind))
                        .src_paths(src_paths);
                    pull(&payload);
                    mqtt_client.publish(&mut payload)
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
                    if !filtered_paths.is_empty() {
                        *cycle += 1;
                        let mut payload =
                            ipc::Payload::new()?.src_paths(filtered_paths).cycle(*cycle);
                        if let Err(e) = mqtt_client.publish(&mut payload) {
                            error!("unable to publish {}", e);
                        } else {
                            info!("published payload: {}", payload);
                        }
                    } else {
                        debug!("client:process>> nothing to send");
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
                Err(err) => error!("watch error: {:?}", err),
            },
            notify::Config::default().with_poll_interval(std::time::Duration::from_secs(1)), // Set polling interval
        )
        .expect("couldn't create watcher");

        if watcher.watch(pathbuf, RecursiveMode::Recursive).is_err() {
            warn!("unable to set watcher for: '{}'", pathbuf.display());
            continue;
        }

        info!("set watcher for: '{}'", pathbuf.display());
        watchers.push(watcher);
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
    rsync(&payload.src_paths, &dest);
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

    rsync(&srcs, &payload.dest_path);
}

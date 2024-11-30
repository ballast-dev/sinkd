use crossbeam::channel::TryRecvError;
use notify::{DebouncedEvent, Watcher};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use crate::{bad, config, ipc, outcome::Outcome, parameters::Parameters};

static FATAL_FLAG: AtomicBool = AtomicBool::new(false);

pub fn start(params: &Parameters) -> Outcome<()> {
    ipc::start_mosquitto()?;
    ipc::daemon(init, "client", params)
}

pub fn stop(params: &Parameters) -> Outcome<()> {
    ipc::end_process(params)
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

fn init(params: &Parameters) -> Outcome<()> {
    let (srv_addr, mut inode_map) = config::get(params)?;

    let (notify_tx, notify_rx): (mpsc::Sender<DebouncedEvent>, mpsc::Receiver<DebouncedEvent>) =
        mpsc::channel();
    let (event_tx, event_rx): (mpsc::Sender<PathBuf>, mpsc::Receiver<PathBuf>) = mpsc::channel();

    // watch_thread needs a mutable map to assign "last event" to inode
    // after config loads up the inode map it is treated as Read Only
    let inode_map2 = inode_map.clone();

    // keep the watchers alive!
    let _watchers = match get_watchers(&inode_map, notify_tx) {
        Ok(w) => w,
        Err(e) => return bad!("{}", e),
    };

    // watch file events
    let watch_thread =
        thread::spawn(move || watch_entry(&mut inode_map, notify_rx, event_tx, &FATAL_FLAG));

    // listen to messages from server
    let mqtt_thread =
        thread::spawn(
            move || match mqtt_entry(&srv_addr, &inode_map2, event_rx, &FATAL_FLAG) {
                Ok(()) => Ok(()),
                Err(e) => {
                    error!("client>> FATAL condition in mqtt_entry, {}", e);
                    Err(e)
                }
            },
        );

    watch_thread.join().unwrap();
    match mqtt_thread.join().unwrap() {
        Err(e) => Err(e),
        Ok(()) => Ok(()),
    }
}

// This will check the event path against the known paths passed at config time
// Only top level paths are sent to the synch thread if the watched directory has exceeded
// interval. In other words events are filtered against intervals (per inode) and added
// to the synch queue.
fn check_interval(
    event_path: &Path,
    inode_map: &mut config::InodeMap,
    event_tx: &mpsc::Sender<PathBuf>,
) {
    // need to dynamically lookup keys and compare path names
    debug!("checking interval, event:{}", event_path.display());
    for (inode_path, inode) in inode_map {
        if event_path.starts_with(inode_path) {
            let now = Instant::now();
            let elapse = now.duration_since(inode.last_event);
            if elapse >= inode.interval {
                debug!("EVENT>> elapse: {}", elapse.as_secs());
                inode.last_event = now;
                event_tx.send(inode_path.clone()).unwrap(); // to kick off synch thread
            }
            // find parent folder and let rsync delta algorithm handle the rest
            break;
        }
    }
}

fn watch_entry(
    inode_map: &mut config::InodeMap,
    notify_rx: mpsc::Receiver<DebouncedEvent>,
    event_tx: mpsc::Sender<PathBuf>,
    fatal_flag: &AtomicBool,
) {
    loop {
        if fatal_flag.load(Ordering::SeqCst) {
            break;
        }

        // blocking call
        match notify_rx.try_recv() {
            Ok(event) => match event {
                DebouncedEvent::Create(path)
                | DebouncedEvent::Write(path)
                | DebouncedEvent::Chmod(path)
                | DebouncedEvent::Remove(path)
                | DebouncedEvent::Rename(path, _) => check_interval(&path, inode_map, &event_tx),
                DebouncedEvent::Rescan
                | DebouncedEvent::NoticeWrite(_)
                | DebouncedEvent::NoticeRemove(_) => {}
                DebouncedEvent::Error(error, option_path) => {
                    info!(
                        "What was the error? {:?}\n the path should be: {:?}",
                        error.to_string(),
                        option_path.unwrap()
                    );
                }
            },
            Err(err) => match err {
                mpsc::TryRecvError::Empty => {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                mpsc::TryRecvError::Disconnected => {
                    error!("FATAL: notify_rx hung up in watch_entry");
                    fatal_flag.store(true, Ordering::SeqCst);
                }
            },
        }
    }
}

fn mqtt_entry(
    server_addr: &str,
    inode_map: &config::InodeMap,
    event_rx: mpsc::Receiver<PathBuf>,
    fatal_flag: &AtomicBool,
) -> Outcome<()> {
    let _payload = ipc::Payload::new();

    let (mqtt_client, mqtt_rx): (ipc::MqttClient, ipc::Rx) =
        match ipc::MqttClient::new(Some(server_addr), &["sinkd/server"], "sinkd/clients") {
            Ok((client, rx)) => (client, rx),
            Err(e) => {
                fatal_flag.store(true, Ordering::SeqCst);
                return bad!("Unable to create mqtt client, {}", e);
            }
        };

    //// assume we are behind
    //let mut server_status = ipc::Status::NotReady(ipc::Reason::Behind);
    let mut cycle: u32 = 0; // WARN: maybe this should be read from disk

    // The server will send status updates to it's clients every 5 seconds
    loop {
        if fatal_flag.load(Ordering::SeqCst) {
            return bad!("client:mqtt_entry>> exit condition reached in watch thread");
        }

        match mqtt_rx.try_recv() {
            Ok(message) => {
                if let Some(msg) = message {
                    if let Ok(decoded_payload) = ipc::decode(msg.payload()) {
                        // process mqtt traffic from server
                        debug!("client>> 👍 recv: {}", decoded_payload);
                        if let Err(e) = process(
                            &event_rx,
                            &mqtt_client,
                            inode_map,
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
                    fatal_flag.store(true, Ordering::SeqCst);
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
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn process(
    event_rx: &mpsc::Receiver<PathBuf>,
    mqtt_client: &ipc::MqttClient,
    inode_map: &config::InodeMap,
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
                let mut payload = ipc::Payload::new()?
                    .status(ipc::Status::NotReady(ipc::Reason::Behind))
                    .src_paths(inode_map.keys().cloned().collect());

                pull(&payload);
                mqtt_client.publish(&mut payload)
            }

            ipc::Reason::Other => todo!(),
        },
        ipc::Status::Ready => {
            debug!("client:process>> ipc::Status::Ready");
            match filter_file_events(event_rx) {
                Ok(filtered_paths) => {
                    if filtered_paths.len() > 0 {
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
                    return bad!("unable to filter_paths: {}", e);
                }
            }
        }
    }
}

fn get_watchers(
    inode_map: &config::InodeMap,
    tx: mpsc::Sender<notify::DebouncedEvent>,
) -> Outcome<Vec<notify::RecommendedWatcher>> {
    let mut watchers: Vec<notify::RecommendedWatcher> = Vec::new();

    for pathbuf in inode_map.keys() {
        //TODO: use 'system_interval' to setup notification events
        let mut watcher =
            notify::watcher(tx.clone(), Duration::from_secs(1)).expect("couldn't create watch");

        if watcher
            .watch(pathbuf, notify::RecursiveMode::Recursive)
            .is_err()
        {
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
    ipc::rsync(&payload.src_paths, &dest);
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

    ipc::rsync(&srcs, &payload.dest_path);
}

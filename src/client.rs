use crate::{config, ipc, shiplog, utils};
use crossbeam::channel::TryRecvError;
use notify::{DebouncedEvent, Watcher};
use paho_mqtt as mqtt;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[warn(unused_features)]
pub fn start(verbosity: u8, clear_logs: bool) -> Result<(), String> {
    shiplog::init(clear_logs)?;
    let (srv_addr, mut inode_map) = config::get()?;

    let (notify_tx, notify_rx): (mpsc::Sender<DebouncedEvent>, mpsc::Receiver<DebouncedEvent>) =
        mpsc::channel();
    let (synch_tx, synch_rx): (mpsc::Sender<PathBuf>, mpsc::Receiver<PathBuf>) = mpsc::channel();

    let exit_cond = Arc::new(Mutex::new(false));
    let exit_cond2 = Arc::clone(&exit_cond);

    // keep the watchers alive!
    let _watchers = get_watchers(&inode_map, notify_tx);

    let watch_thread = thread::spawn(move || {
        watch_entry(&mut inode_map, notify_rx, synch_tx, &*exit_cond);
    });

    let synch_thread = thread::spawn(move || {
        if let Err(error) = mqtt_entry(&srv_addr, synch_rx, &*exit_cond2) {
            utils::fatal(&*exit_cond2);
            error!("FATAL condition in mqtt thread, {}", error);
        }
    });

    if let Err(error) = watch_thread.join() {
        return Err(format!("Client watch thread error! {:?}", error));
    }
    if let Err(error) = synch_thread.join() {
        return Err(format!("Client synch thread error! {:?}", error));
    }

    Ok(())

    // TODO: need packager to setup file with correct permisions
    // let daemon = Daemonize::new()
    //     .pid_file(utils::PID_PATH)
    //     .group("sinkd");
    // .chown_pid_file(true)  // is optional, see `Daemonize` documentation
    // .user("nobody")

    // match daemon.start() {
    //     Ok(_) => {
    //         info!("about to start daemon...");
    //         run();
    //     }
    //     Err(e) => error!("sinkd did not start (already running?), {}", e),
    // }
}

fn update(event_path: PathBuf, inode_map: &mut config::InodeMap, synch_tx: &mpsc::Sender<PathBuf>) {
    for (inode_path, inode) in inode_map {
        if event_path.starts_with(inode_path) {
            let now = Instant::now();
            let elapse = now.duration_since(inode.last_event);
            if elapse >= inode.interval {
                debug!("EVENT>> elapse: {}", elapse.as_secs());
                inode.last_event = now;
                synch_tx.send(inode_path.clone()).unwrap(); // to kick off synch thread
            }
            break;
        }
    }
}

fn watch_entry(
    inode_map: &mut config::InodeMap,
    notify_rx: mpsc::Receiver<DebouncedEvent>,
    synch_tx: mpsc::Sender<PathBuf>,
    exit_cond: &Mutex<bool>,
) {
    loop {
        if utils::exited(&exit_cond) {
            break;
        }

        match notify_rx.try_recv() {
            // blocking call
            Ok(event) => match event {
                DebouncedEvent::Create(path) => update(path, inode_map, &synch_tx),
                DebouncedEvent::Write(path) => update(path, inode_map, &synch_tx),
                DebouncedEvent::Chmod(path) => update(path, inode_map, &synch_tx),
                DebouncedEvent::Remove(path) => update(path, inode_map, &synch_tx),
                DebouncedEvent::Rename(path, _) => update(path, inode_map, &synch_tx),
                DebouncedEvent::Rescan => {}
                DebouncedEvent::NoticeWrite(_) => {}
                DebouncedEvent::NoticeRemove(_) => {}
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
                    std::thread::sleep(std::time::Duration::from_millis(200))
                }
                mpsc::TryRecvError::Disconnected => {
                    error!("FATAL: notify_rx hung up in watch_entry");
                    utils::fatal(&exit_cond);
                }
            },
        }
    }
}

fn mqtt_entry(
    server_addr: &str,
    synch_rx: mpsc::Receiver<PathBuf>,
    exit_cond: &Mutex<bool>,
) -> Result<(), mqtt::Error> {
    let mut payload = ipc::Payload::new();
    // Using Hashset to prevent repeated entries
    let mut events = HashSet::new();
    let mut status = ipc::Status::Sinkd;
    // TODO need to read from config

    let (mqtt_client, mqtt_rx) = ipc::MqttClient::new(
        Some(server_addr), 
        &["sinkd/server"],
        "sinkd/clients"
    )?;

    loop {
        // Check to make sure other thread didn't exit
        if utils::exited(&exit_cond) {
            return Err(mqtt::Error::General("exit condition reached"));
        }

        // process mqtt traffic from server
        match mqtt_rx.try_recv() {
            // hope that no messages are lost...
            Ok(message) => {
                if let Some(msg) = message {
                    debug!("client got message!: {}", msg);
                    let pyld = ipc::decode(msg.payload())?;
                    match pyld.status {
                        ipc::Status::Sinkd => {} // server should never send this
                        ipc::Status::Edits => {} // server should never send this,
                        ipc::Status::Cache => {
                            // server is telling us to sinkd::cache()
                        }
                        ipc::Status::Behind => {}, // server is telling us to update?
                        ipc::Status::Updating => {
                            // if the server is Updating dont check cycle number
                            // may need to track state
                        }
                    }
                }
            }
            Err(e) => match e {
                TryRecvError::Disconnected => return Err(mqtt::Error::General("mqtt_rx hung up?")),
                TryRecvError::Empty => (),
            },
        }

        // if received {
        //     let timestamp = utils::get_timestamp("%Y%m%d");
        //     let payload = ipc::Payload::from(
        //         &hostname,
        //         &username,
        //         &timestamp,
        //         std::str::from_utf8(curr_msg.payload()).unwrap(),
        //         1,
        //         ipc::Status::Sinkd,
        //     );
        //     mqtt_client.publish(mqtt::Message::new(
        //         "sinkd/server",
        //         ipc::encode(payload)?,
        //         mqtt::QOS_1,
        //     ))?;
        // }

        // process file events
        match synch_rx.try_recv() {
            Ok(path) => {
                debug!("received from synch channel");
                events.insert(path);
            }
            Err(_) => {
                // buffered events
                if !events.is_empty() {
                    for path in events.drain() {
                        // TODO: need to account for serveral users
                        payload.path = String::from(path.to_str().unwrap());
                        payload.cycle += 1;
                        payload.status = ipc::Status::Edits;
                        mqtt_client.publish(&mut payload)?;
                    }
                }
            }
        }
        // TODO: add 'system_interval' to config
        std::thread::sleep(Duration::from_secs(1));
        debug!("synch loop...")
    }
    // Err(e) => {
    //     error!("FATAL: unable to create MQTT client, {}", e);
    //     utils::fatal(&exit_cond);
    // }
}
fn get_watchers(
    inode_map: &config::InodeMap,
    tx: mpsc::Sender<notify::DebouncedEvent>,
) -> Vec<notify::RecommendedWatcher> {
    let mut watchers: Vec<notify::RecommendedWatcher> = Vec::new();

    for (pathbuf, _) in inode_map.iter() {
        //TODO: use 'system_interval' to setup notification events
        let mut watcher =
            notify::watcher(tx.clone(), Duration::from_secs(1)).expect("couldn't create watch");

        match watcher.watch(pathbuf, notify::RecursiveMode::Recursive) {
            Err(_) => {
                warn!("unable to set watcher for: '{}'", pathbuf.display());
                continue;
            }
            Ok(_) => {
                info!("set watcher for: '{}'", pathbuf.display());
                watchers.push(watcher);
            }
        }
    }
    return watchers;
}

fn cache(path: &str) -> Result<(), String> {
    Ok(())
}

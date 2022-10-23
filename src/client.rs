use crate::{config, defs::Outcome, ipc, shiplog, utils};
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

pub enum State {
    Caching,
    Sinking,
    Behind,
    UpToDate,
}

#[warn(unused_features)]
pub fn start(verbosity: u8, clear_logs: bool) -> Result<(), String> {
    shiplog::init(clear_logs)?;
    let (srv_addr, mut inode_map) = config::get()?;

    let (notify_tx, notify_rx): (mpsc::Sender<DebouncedEvent>, mpsc::Receiver<DebouncedEvent>) =
        mpsc::channel();
    let (event_tx, event_rx): (mpsc::Sender<PathBuf>, mpsc::Receiver<PathBuf>) = mpsc::channel();

    let exit_cond = Arc::new(Mutex::new(false));
    let exit_cond2 = Arc::clone(&exit_cond);

    // keep the watchers alive!
    let _watchers = get_watchers(&inode_map, notify_tx);

    let watch_thread = thread::spawn(move || {
        watch_entry(&mut inode_map, notify_rx, event_tx, &*exit_cond);
    });

    let mqtt_thread = thread::spawn(move || {
        if let Err(error) = mqtt_entry(&srv_addr, event_rx, &*exit_cond2) {
            utils::fatal(&*exit_cond2);
            error!("client>> FATAL condition in mqtt_entry, {}", error);
        }
    });

    if let Err(error) = watch_thread.join() {
        return Err(format!("Client watch thread error! {:?}", error));
    }
    if let Err(error) = mqtt_thread.join() {
        return Err(format!("Client mqtt thread error! {:?}", error));
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

// This will check the event path against the known paths passed at config time
// Only top level paths are sent to the synch thread if the watched directory has exceeded
// interval. In other words events are filtered against intervals (per inode) and added
// to the synch queue.
fn interval_add(
    event_path: PathBuf,
    inode_map: &mut config::InodeMap,
    synch_tx: &mpsc::Sender<PathBuf>,
) {
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
    event_tx: mpsc::Sender<PathBuf>,
    exit_cond: &Mutex<bool>,
) {
    loop {
        if utils::exited(&exit_cond) {
            break;
        }

        match notify_rx.try_recv() {
            // blocking call
            Ok(event) => match event {
                DebouncedEvent::Create(path) => interval_add(path, inode_map, &event_tx),
                DebouncedEvent::Write(path) => interval_add(path, inode_map, &event_tx),
                DebouncedEvent::Chmod(path) => interval_add(path, inode_map, &event_tx),
                DebouncedEvent::Remove(path) => interval_add(path, inode_map, &event_tx),
                DebouncedEvent::Rename(path, _) => interval_add(path, inode_map, &event_tx),
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
    event_rx: mpsc::Receiver<PathBuf>,
    exit_cond: &Mutex<bool>,
) -> Outcome<()> {
    let mut payload = ipc::Payload::new();
    // Using Hashset to prevent repeated entries
    let mut state = State::UpToDate;
    // TODO need to read from config

    let (mqtt_client, mqtt_rx) =
        ipc::MqttClient::new(Some(server_addr), &["sinkd/server"], "sinkd/clients")?;

    loop {
        // Check to make sure other thread didn't exit
        if utils::exited(&exit_cond) {
            return Err(mqtt::Error::General("exit condition reached"));
        }

        // process mqtt traffic from server
        // first check latest message from server then send payload
        match mqtt_rx.try_recv() {
            // hope that no messages are lost...
            Ok(message) => {
                if let Some(msg) = message {
                    debug!("client>> got message!: {}", msg);
                    let pyld = ipc::decode(msg.payload())?;
                    match pyld.status {
                        ipc::Status::NotReady(reason) => match reason {
                            ipc::Reason::Sinking => {
                                std::thread::sleep(Duration::from_secs(5));
                            }
                            ipc::Reason::Behind => {
                                // spawn rsync on the client?
                                // wait for server to be ready then clone down
                            }
                            ipc::Reason::Other => todo!(),
                        },
                        ipc::Status::Ready => {
                            payload.paths = filter_file_events(&event_rx)?;
                            payload.hostname = utils::get_hostname();
                            payload.username = utils::get_username();
                            payload.date = utils::get_timestamp("%Y%m%d");
                            payload.cycle += 1;
                            payload.status = ipc::Status::Ready;
                            mqtt_client.publish(&mut payload)?;
                        }
                    }
                } else {
                    error!("client>> mqtt_thread: empty message?");
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

// Will loop on file events until queue (channel) is empty
// Using a HashSet to filter out redundancies will return
// sanitized list of paths ready to send to sinkd server
// TODO: need to account for serveral users
fn filter_file_events(event_rx: &mpsc::Receiver<PathBuf>) -> Outcome<Vec<String>> {
    let mut path_set = HashSet::<PathBuf>::new();
    loop {
        match event_rx.try_recv() {
            // only top level paths are passed
            Ok(path) => {
                debug!("Adding file to path_set");
                path_set.insert(path);
            }
            Err(err) => match err {
                mpsc::TryRecvError::Disconnected => {
                    return Err(mqtt::Error::General("event_rx disconnected"))
                }
                mpsc::TryRecvError::Empty => break, // Ready to send!
            },
        }
    }

    // buffered events
    let mut event_paths = vec![];
    if !path_set.is_empty() {
        for path in path_set.drain() {
            event_paths.push(String::from(path.to_str().unwrap()));
        }
    }
    Ok(event_paths)
}

// let ep = vec![ path_set.drain().map(|s| String::from(s.to_str().unwrap())).collect() ];

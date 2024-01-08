use crate::{
    config, 
    ipc, 
    outcome::Outcome, 
    shiplog, 
    utils::{self, Parameters}
};
use crossbeam::channel::TryRecvError;
use notify::{DebouncedEvent, Watcher};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

static FATAL_FLAG: AtomicBool = AtomicBool::new(false);

#[warn(unused_features)]
pub fn start(params: &Parameters) -> Outcome<()> {
    // TODO: need packager to setup file with correct permisions
    shiplog::init(params)?;
    utils::start_mosquitto()?;
    utils::daemon(init, "client", params)
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

    let watch_thread =
        thread::spawn(move || watch_entry(&mut inode_map, notify_rx, event_tx, &FATAL_FLAG));

    let mqtt_thread =
        thread::spawn(
            move || match mqtt_entry(&srv_addr, &inode_map2, event_rx, &FATAL_FLAG) {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("client>> FATAL condition in mqtt_entry, {}", e);
                    Err(e)
                }
            },
        );

    watch_thread.join().unwrap();
    match mqtt_thread.join().unwrap() {
        Err(e) => Err(e),
        Ok(_) => Ok(()),
    }
}

// This will check the event path against the known paths passed at config time
// Only top level paths are sent to the synch thread if the watched directory has exceeded
// interval. In other words events are filtered against intervals (per inode) and added
// to the synch queue.
fn check_interval(
    event_path: PathBuf,
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

        match notify_rx.try_recv() {
            // blocking call
            Ok(event) => match event {
                DebouncedEvent::Create(path) => check_interval(path, inode_map, &event_tx),
                DebouncedEvent::Write(path) => check_interval(path, inode_map, &event_tx),
                DebouncedEvent::Chmod(path) => check_interval(path, inode_map, &event_tx),
                DebouncedEvent::Remove(path) => check_interval(path, inode_map, &event_tx),
                DebouncedEvent::Rename(path, _) => check_interval(path, inode_map, &event_tx),
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
    let (mqtt_client, mqtt_rx): (ipc::MqttClient, ipc::Rx);
    match ipc::MqttClient::new(Some(server_addr), &["sinkd/server"], "sinkd/clients") {
        Ok((client, rx)) => {
            mqtt_client = client;
            mqtt_rx = rx;
        }
        Err(e) => {
            fatal_flag.store(true, Ordering::SeqCst);
            return bad!("Unable to create mqtt client, {}", e);
        }
    }

    // The server will send status updates to it's clients every 5 seconds
    loop {
        if fatal_flag.load(Ordering::SeqCst) {
            return bad!("mqtt_entry>> exit condition reached in watch thread");
        }

        // process mqtt traffic from server
        // first check latest message from server then send payload
        match mqtt_rx.try_recv() {
            Ok(message) => {
                if let Some(msg) = message {
                    debug!("client>> got message!: {}", msg);
                    if let Ok(decoded_payload) = ipc::decode(msg.payload()) {
                        // recieved message from server, need to process
                        process(&event_rx, &mqtt_client, inode_map, decoded_payload);
                    } else {
                        error!("unable to decode message: {:?}", msg.payload())
                    }
                } else {
                    error!("client>> mqtt_thread: empty message?");
                }
            }
            Err(e) => match e {
                TryRecvError::Disconnected => {
                    fatal_flag.store(true, Ordering::SeqCst);
                    return bad!("mqtt_rx hung up?");
                }
                TryRecvError::Empty => {
                    debug!("waiting on message...")
                }
            },
        }

        // TODO: add 'system_interval' to config
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn process(
    event_rx: &mpsc::Receiver<PathBuf>,
    mqtt_client: &ipc::MqttClient,
    inode_map: &config::InodeMap,
    server_payload: ipc::Payload,
) {
    // process received message from server

    match server_payload.status {
        ipc::Status::NotReady(reason) => match reason {
            ipc::Reason::Sinking => {
                std::thread::sleep(Duration::from_secs(5)); // should be config driven
            }
            ipc::Reason::Behind => {
                // spawn rsync on the client?
                // better to spawn on server to keep things in "lock step"
                let _paths: Vec<&PathBuf> = inode_map.keys().collect();

                // utils::rsync(&ipc::Payload::new().paths(*inode_map.keys().collect::<PathBuf>());

                // if let Err(e) = mqtt_client.publish(
                //     &ipc::Payload::new().status(ipc::Status::NotReady(ipc::Reason::Behind))
                // ) {
                //     error!("client>> couldn't publish Behind status, {}", e);
                // }
            }
            ipc::Reason::Other => todo!(),
        },
        ipc::Status::Ready => {
            let mut payload = ipc::Payload::new();
            match filter_file_events(event_rx) {
                Ok(filtered_paths) => payload.paths = filtered_paths,
                Err(e) => error!("{}", e),
            }
            payload.cycle += 1;
            if let Err(e) = mqtt_client.publish(&mut payload) {
                error!("unable to publish {}", e);
            }
        }
    }
}

fn get_watchers(
    inode_map: &config::InodeMap,
    tx: mpsc::Sender<notify::DebouncedEvent>,
) -> Outcome<Vec<notify::RecommendedWatcher>> {
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

    if watchers.len() > 0 {
        Ok(watchers)
    } else {
        bad!("nothing to watch! aborting")
    }
}

fn cache(_path: &str) -> Result<(), String> {
    Ok(())
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

// let ep = vec![ path_set.drain().map(|s| String::from(s.to_str().unwrap())).collect() ];

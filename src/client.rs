use crate::{config, ipc, shiplog, utils};
use notify::{DebouncedEvent, Watcher};
use paho_mqtt as mqtt;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// Both macOS and Linux have the uname command
fn get_hostname() -> String {
    match std::process::Command::new("uname").arg("-n").output() {
        Err(e) => {
            error!("uname didn't work? {}", e);
            String::from("uname-error")
        }
        Ok(output) => String::from_utf8(output.stdout.to_ascii_lowercase()).unwrap_or_else(|_| {
            error!("invalid string from uname -a");
            String::from("invalid-hostname")
        }),
    }
}

/// Both macOS and Linux have the whoami command
fn get_username() -> String {
    match std::process::Command::new("whoami").output() {
        Err(e) => {
            error!("whoami didn't work? {}", e);
            String::from("whoami error")
        }
        Ok(output) => String::from_utf8(output.stdout.to_ascii_lowercase()).unwrap_or_else(|_| {
            error!("invalid string from whoami");
            String::from("invalid-username")
        }),
    }
}

fn dispatch(msg: &Option<mqtt::Message>) {
    if let Some(msg) = msg {
        let payload = std::str::from_utf8(msg.payload()).unwrap();
        debug!("topic: {}\tpayload: {}", msg.topic(), payload);
    } else {
        error!("malformed mqtt message");
    }
}

#[warn(unused_features)]
pub fn start(verbosity: u8, clear_logs: bool) -> bool {
    if let Err(e) = shiplog::init(clear_logs) {
        eprintln!("{}", e);
        return false;
    }

    let (srv_addr, mut inode_map) = config::get();
    let (notify_tx, notify_rx): (mpsc::Sender<DebouncedEvent>, mpsc::Receiver<DebouncedEvent>) =
        mpsc::channel();
    let (synch_tx, synch_rx): (mpsc::Sender<PathBuf>, mpsc::Receiver<PathBuf>) = mpsc::channel();

    // keep the watchers alive!
    let _watchers = get_watchers(&inode_map, notify_tx);

    let watch_thread = thread::spawn(move || {
        watch_entry(&mut inode_map, notify_rx, synch_tx);
    });

    let synch_thread = thread::spawn(move || {
        mqtt_entry(&srv_addr, synch_rx);
    });

    if let Err(_) = watch_thread.join() {
        error!("Client watch thread error!");
        return false;
    }
    if let Err(_) = synch_thread.join() {
        error!("Client synch thread error!");
        return false;
    }

    true

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
) {
    loop {
        match notify_rx.recv() {
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
            Err(e) => {
                error!("FATAL: notify mpsc::channel hung up in watch_entry {:?}", e);
                panic!()
            }
        }
    }
}

fn mqtt_entry(server_addr: &String, synch_rx: mpsc::Receiver<PathBuf>) {
    let hostname = get_hostname();
    let username = get_username();
    // TODO need to read from config
    match ipc::MqttClient::new(Some(server_addr), dispatch) {
        Ok(mut mqtt) => {
            mqtt.subscribe("sinkd/server");
            // Using Hashset to prevent repeated entries
            let mut events = HashSet::new();

            loop {
                //? STATE MACHINE
                // 1. file event
                // 2. query server for status
                // 3. if not up to date, update
                // -- update algorithm --
                // - 1. rsync from server files into .sinkd/ (per directory)
                // - 2. compare files (stat on modify field?) (use meta data?)
                // - 3. conflicted files will be marked as file.sinkd
                // - 4. move everython from .sinkd/ to correct dir
                // - 5. remove .sinkd/
                // - 6. make sinkd_num same as server
                // 4. send mqtt message topic=sinkd/update/client_name payload=dir,dir,dir...
                // 5. server calls rsync src=client dest=server
                // 6. server increments sinkd_num
                // 7. server mqtt publishes topic=sinkd/status payload=sinkd_num
                match synch_rx.try_recv() {
                    Ok(path) => {
                        debug!("received from synch channel");
                        events.insert(path);
                    }
                    Err(_) => {
                        // buffered events
                        if !events.is_empty() {
                            for path in events.drain() {
                                if let Ok(packet) = ipc::packed(
                                    &hostname,
                                    &username,
                                    &path.to_str().unwrap(),
                                    &utils::get_timestamp("%Y%m%d"),
                                    1,
                                    ipc::Status::Edits,
                                ) {
                                    mqtt.publish(mqtt::Message::new(
                                        "sinkd/client",
                                        packet,
                                        mqtt::QOS_0,
                                    ));
                                }
                            }
                        }
                        // TODO: add 'system_interval' to config
                        std::thread::sleep(Duration::from_secs(1));
                        debug!("synch loop...")
                    }
                }
            }
        }
        Err(e) => {
            error!("FATAL: unable to create MQTT client, {}", e);
            // TODO: need to handle closing of multiple threads and print out to user fatal errors
            // TODO: on stderr 
        }
    }
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

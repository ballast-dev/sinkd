use std::{
    collections::HashSet,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant}
};
use notify::{DebouncedEvent, Watcher};
use paho_mqtt as mqtt;
use crate::{
    config,
    protocol,
    shiplog,
    utils
};


fn init() -> Result<(), String> {
    match utils::create_log_file() {
        Err(e) => Err(e),
        Ok(_) => { 
            shiplog::ShipLog::init(); 
            match utils::create_pid_file() {
                Err(e) => Err(e),
                Ok(_) => Ok(())
            }
        }
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

/**
 * When sinkd is packaged should install /run/sinkd.pid file and make it writable the the sinkd group
 * Need to set up logging keep everything local to home directory ~/
 */
// #[warn(unused_features)]
pub fn start(verbosity: u8) -> bool {

    if let Err(e) = init() {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    // notify has a watch thread to watch to the files (endless loop)
    // notify has a synch thread to rsync the changes (endless loop)
    let notify_thread = thread::spawn(move || notify_entry());

    if let Err(_) = notify_thread.join() {
        error!("Client synch thread error!");
        std::process::exit(1);
    }

    return true;

    

    // // TODO need to read from config
    // if let Ok(_) = protocol::MqttClient::new(Some("localhost"), dispatch) {

    // } else {
    //     return false;
    // }




    // // TODO: need packager to setup file with correct permisions
    // let daemon = Daemonize::new()
    //     .pid_file(utils::PID_PATH)
    //     .group("sinkd");
    //     // .chown_pid_file(true)  // is optional, see `Daemonize` documentation
    //     // .user("nobody")

    // match daemon.start() {
    //     Ok(_) => {
    //         info!("about to start daemon...");
    //         run();
    //     }
    //     Err(e) => error!("sinkd did not start (already running?), {}", e),
    // }
}


fn notify_entry() {
    // make this more robust
    //? have Config:get_inode_map() -> Result<HashMap, Config::Error> {}
    //? be the main entry call
    //? Config should panic and do all error handling within module

    let (srv_addr, mut inode_map) = config::get();

    let (notify_tx, notify_rx) = mpsc::channel();
    // let (synch_tx, synch_rx): (mpsc::Sender::<PathBuf>, mpsc::Receiver::<PathBuf>) = mpsc::channel();
    let (synch_tx, synch_rx) = mpsc::channel(); // just pass type for compiler to understand
    
    // keep the watchers alive!
    let _watchers = get_watchers(&inode_map, notify_tx);

    let watch_thread = thread::spawn(move || {
        watch_entry(&mut inode_map, notify_rx, synch_tx);
    });

    let synch_thread = thread::spawn(move || {
        synch_entry(&srv_addr, synch_rx);
    });
    
    if let Err(_) = watch_thread.join() {
        error!("Client watch thread error!");
        std::process::exit(1);
    }
    if let Err(_) = synch_thread.join() {
        error!("Client synch thread error!");
        std::process::exit(1);
    }
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

fn watch_entry(inode_map: &mut config::InodeMap, notify_rx: mpsc::Receiver<DebouncedEvent>, synch_tx: mpsc::Sender<PathBuf>) {
    loop {
        match notify_rx.recv() { // blocking call
            Ok(event) => {
                debug!("received watch event!");
                match event {
                    DebouncedEvent::NoticeWrite(_) => {}  // do nothing
                    DebouncedEvent::NoticeRemove(_) => {} // do nothing for notices
                    DebouncedEvent::Create(path) => update(path, inode_map, &synch_tx),
                    DebouncedEvent::Write(path) => update(path, inode_map, &synch_tx),
                    DebouncedEvent::Chmod(path) => update(path, inode_map, &synch_tx),
                    DebouncedEvent::Remove(path) => update(path, inode_map, &synch_tx),
                    DebouncedEvent::Rename(path, _) => update(path, inode_map, &synch_tx),
                    DebouncedEvent::Rescan => {},
                    DebouncedEvent::Error(error, option_path) => {
                        info!(
                            "What was the error? {:?}\n the path should be: {:?}",
                            error.to_string(),
                            option_path.unwrap()
                        );
                    }
                }
            }
            Err(e) => {
                info!("notify mpsc::channel hung up... {:?}", e);
                thread::sleep(Duration::from_secs(2))
            }
        }
        // TODO: to sleep on interval pulled from configuration
        // std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

fn synch_entry(server_addr: &String, synch_rx: mpsc::Receiver<PathBuf>) {
    // Aggregate the calls under the parent folder, to minimize overhead

    //! need to pull excludes from config on loaded path
    //! '/srv/sinkd/user' will have permissions of user (to prevent rsync errors)
    //? RSYNC options to consider
    //~ --copy-links  (included with -a, copies where it points to)
    //~ --delete (must be a whole directory, no wildcards)
    //~ --delete-excluded (also delete excluded files)
    //~ --max-size=SIZE (limit size of transfers)
    //~ --exclude 

    // let now = Instant::now();
    // thread::sleep(Duration::new(1, 0));
    // let new_now = Instant::now();
    // println!("{:?}", new_now.checked_duration_since(now));
    // println!("{:?}", now.checked_duration_since(new_now)); // None

    let mut events = HashSet::new();

    loop {
        match synch_rx.try_recv() {

            Ok(path) => {
                debug!("received from synch channel");
                events.insert(path);
            },
            Err(_) => { // buffered events

                if !events.is_empty() {
                    for path in events.drain() {
                        fire_rsync(server_addr, &path);
                    }
                }
               
                std::thread::sleep(Duration::from_secs(1)); // "system_interval" config value
                debug!("synch loop...")
            }
        }

    }

}

fn get_watchers(inode_map: &config::InodeMap, tx: mpsc::Sender<notify::DebouncedEvent>) -> Vec<notify::RecommendedWatcher> {
    let mut watchers: Vec<notify::RecommendedWatcher> = Vec::new();

    for (pathbuf, inode) in inode_map.iter() {
        let mut watcher = notify::watcher(tx.clone(), inode.interval).expect("couldn't create watch");

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

fn fire_rsync(hostname: &String, src_path: &PathBuf) {
    // debug!("username: {}, hostname: {}, path: {}", username, hostname, path.display());

    // Agnostic pathing allows sinkd not to care about user folder structure
    let dest_path: String;
    if hostname.starts_with('/') {
        dest_path = String::from("/srv/sinkd/");
    } else {
        // user permissions should persist regardless
        dest_path = format!("sinkd@{}:/srv/sinkd/", &hostname);
    }

    let rsync_result = std::process::Command::new("rsync")
        .arg("-atR") // archive, timestamps, relative
        .arg("--delete")
        .arg(&src_path)
        .arg(&dest_path)
        .spawn();

    match rsync_result {
        Err(x) => {
            error!("{:?}", x);
        }
        Ok(_) => {
            info!("DID IT>> Called rsync src:{}  ->  dest:{}", 
                    &src_path.display(), 
                    &dest_path);
        }
    }
}

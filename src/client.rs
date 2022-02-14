use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant}
};
use notify::{DebouncedEvent, Watcher};
use paho_mqtt as mqtt;
use crate::{
    config::{Config, UserConfig, SysConfig},
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


    // TODO: spawn two threads. 1: polling thread 2: listening thread
    println!("starting?");
    if let Ok(mut client) = protocol::MqttClient::new(Some("localhost"), dispatch) {
        debug!("connected and set callbacks?");
        println!("Waiting for messages...");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            // if let Ok(val) = rx.try_recv() {
            //     println!("got it! {:?}", val);
            //     // client.disconnect(disconn_opts.clone());
            //     std::process::exit(0);
            // }
        }

        return true;
    } else {
        return false;
    }
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


fn run() {
    let mut config = Config::new();
    if !config.init() {
        error!("FATAL couldn't initialize configurations");
        panic!() 
    }

    let (notify_tx, notify_rx) = mpsc::channel();
    // let (synch_tx, synch_rx): (mpsc::Sender::<PathBuf>, mpsc::Receiver::<PathBuf>) = mpsc::channel();
    let (synch_tx, synch_rx) = mpsc::channel(); // just pass type for compiler to understand
    
    // keep the watchers alive!
    let _watchers = get_watchers(&config, notify_tx);

    let watch_thread = thread::spawn(move || {
        watch_entry(notify_rx, synch_tx);
    });

    let synch_thread = thread::spawn(move || {
        synch_entry(&config.sys, &config.users, synch_rx);
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

fn watch_entry(notify_rx: mpsc::Receiver<DebouncedEvent>, synch_tx: mpsc::Sender<PathBuf>) {
    loop {
        match notify_rx.recv() {
            Ok(event) => {
                debug!("received notification event!");
                match event {
                    DebouncedEvent::NoticeWrite(_) => {}  // do nothing
                    DebouncedEvent::NoticeRemove(_) => {} // do nothing for notices
                    DebouncedEvent::Create(path) => synch_tx.send(path).unwrap(),
                    DebouncedEvent::Write(path) => synch_tx.send(path).unwrap(),
                    DebouncedEvent::Chmod(path) => synch_tx.send(path).unwrap(),
                    DebouncedEvent::Remove(path) => synch_tx.send(path).unwrap(),
                    DebouncedEvent::Rename(path, _) => synch_tx.send(path).unwrap(),
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
        // std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn synch_entry(sys_cfg: &SysConfig, users_map: &HashMap<String, UserConfig>, synch_rx: mpsc::Receiver<PathBuf>) {
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

    // create interval hashmap 
    struct Inode {
        path: PathBuf,
        excludes: Vec::<String>, // holds wildcards
        interval: Duration,
        last_event: Instant,
        uncaught_event: bool 
    }

    let mut inode_map: HashMap<String, Vec<Inode>> = HashMap::new();

    for (name, cfg) in users_map.iter() {
        let mut inodes = Vec::new();
        for anchor in &cfg.anchors {
            inodes.push(
                Inode{
                    path: anchor.path.clone(),
                    excludes: anchor.excludes.clone(),
                    interval: Duration::from_secs(anchor.interval),
                    last_event: Instant::now(),
                    uncaught_event: false
                }
            );
        }
        inode_map.insert(name.clone(), inodes);
    }

    // if / in server addr then local path
    let _srv_addr: String;
    if sys_cfg.server_addr.contains("/") {
        _srv_addr = sys_cfg.server_addr.clone();
    } else {
        _srv_addr = String::from("");
    }

    loop {
        match synch_rx.try_recv() {
            Ok(path) => {
                debug!("received from synch thread");
                let mut found = false;
                // instead of looping should find key in map
                for (name, inodes) in inode_map.iter_mut() {
                    for inode in inodes.iter_mut() {
                        if path.starts_with(&inode.path) {
                            let elapse = Instant::now().checked_duration_since(inode.last_event).unwrap();
                            if elapse >= inode.interval {
                                fire_rsync(&name, &sys_cfg.server_addr, &inode.path)
                            } else {
                                inode.uncaught_event = true;
                            }
                            found = true;  // to prevent futher looping
                            break;
                        }
                    }
                    if found { break; }
                }
            },
            Err(_) => {
                for (name, inodes) in inode_map.iter_mut() {
                    for inode in inodes.iter_mut() {
                        if inode.uncaught_event {
                            debug!("uncaught event!");
                            let elapse = Instant::now().checked_duration_since(inode.last_event).unwrap();
                            if elapse >= inode.interval {
                                fire_rsync(&name, &sys_cfg.server_addr, &inode.path)
                            }    
                        }
                    }
                }
                std::thread::sleep(Duration::from_secs(1));
                debug!("synch loop...")
            }
        }

    }

}

fn get_watchers(config: &Config, tx: mpsc::Sender<notify::DebouncedEvent>) -> Vec<notify::RecommendedWatcher> {
    let mut watchers: Vec<notify::RecommendedWatcher> = Vec::new();
    // Set watcher for share drives
    for anchor in config.sys.shares.iter() {
        let interval = Duration::from_secs(anchor.interval.into());
        let mut watcher = notify::watcher(tx.clone(), interval).expect("couldn't create watch");

        match watcher.watch(anchor.path.clone(), notify::RecursiveMode::Recursive) {
            Err(_) => {
                warn!("unable to set watcher for: '{}'", anchor.path.display());
                continue;
            }
            Ok(_) => {
                info!("set watcher for: '{}'", anchor.path.display());
                watchers.push(watcher);
            }
        }
    }

    // Set watcher for user drives
    for usr_cfg in &config.users {
        // Client runs for all users
        for anchor in &usr_cfg.1.anchors {
            let interval = Duration::from_secs(anchor.interval.into());
            let mut watcher = notify::watcher(tx.clone(), interval).expect("couldn't create watch");

            match watcher.watch(anchor.path.clone(), notify::RecursiveMode::Recursive) {
                Err(_) => {
                    warn!("unable to set watcher for: '{}'", anchor.path.display());
                    continue;
                }
                Ok(_) => {
                    info!("set watcher for: '{}'", anchor.path.display());
                    watchers.push(watcher);
                }
            }
        }
    }
    return watchers;
}

fn fire_rsync(username: &String, hostname: &String, path: &PathBuf) {
    // rsync -avt my/path/ tony@host:/path/to/stuff
    let src = path;
    let dest: String;
    if hostname.starts_with('/') {
        dest = format!("/srv/sinkd/{}/{}", &username, &path.display()); 
    } else {
        dest = format!("{}@{}:/srv/sinkd/{}/{}", &username, &hostname, &username, &path.display());
    }

    let rsync_result = std::process::Command::new("rsync")
        .arg("-at") // archive and timestamps
        .arg(&src)
        .arg(&dest)
        .spawn();

    match rsync_result {
        Err(x) => {
            error!("{:?}", x);
        }
        Ok(_) => {
            info!("Called rsync src:{}  ->  dest:{}", 
                    &src.display(), 
                    &dest);
        }
    }
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use crate::config::{Config, UserConfig, SysConfig};
use notify::{DebouncedEvent, Watcher};

const INTERVAL: u8 = 5; // in seconds

pub fn run() {
    let mut config = Config::new();
    if !config.init() {
        error!("FATAL couldn't initialize configurations");
        panic!() 
    }
    
    let (notify_tx, notify_rx) = mpsc::channel();
    // let (synch_tx, synch_rx): (mpsc::Sender::<PathBuf>, mpsc::Receiver::<PathBuf>) = mpsc::channel();
    let (synch_tx, synch_rx) = mpsc::channel(); // just pass type for compiler to understand
    // let watchers = Vec::new();   // TODO: needed?     
    set_watchers(&config, &notify_tx);

    let watch_thread = thread::spawn(move || {
        watch_entry(notify_rx, synch_tx.clone());
    });

    let synch_thread = thread::spawn(move || {
        synch_entry(&config.sys, &config.users, synch_rx);
    });
    
    if let Err(_) = watch_thread.join() {
        println!("oh no.... watch thread bugged out")
    }
    if let Err(_) = synch_thread.join() {
        println!("oh no.... synch thread bugged out")
    }
}

fn watch_entry(notify_rx: mpsc::Receiver<DebouncedEvent>, synch_tx: mpsc::Sender<PathBuf>) {
    info!("watch_entry");
    loop {
        match notify_rx.recv() {
            Ok(event) => {
                info!("received notification event!");
                match event {
                    // todo: maybe aggregate the calls under the parent folder, to minimize overhead
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
            Err(e) => info!("Received DebouncedEvent error: {:?}", e),
        }
        // TODO: to sleep on interval pulled from configuration
        // std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn synch_entry(sys_cfg: &SysConfig, users_map: &HashMap<String, UserConfig>, synch_rx: mpsc::Receiver<PathBuf>) {
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

    info!("synch_entry");
    loop {
        match synch_rx.try_recv() {
            Ok(path) => {
                info!("received from synch thread");
                let mut found = false;
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
                            info!("uncaught event!");
                            let elapse = Instant::now().checked_duration_since(inode.last_event).unwrap();
                            if elapse >= inode.interval {
                                fire_rsync(&name, &sys_cfg.server_addr, &inode.path)
                            }    
                        }
                    }
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        }

    }

}

fn set_watchers(config: &Config, notify_tx: &mpsc::Sender::<DebouncedEvent>) {

    // Set watcher for share drives
    for anchor in config.sys.shares.iter() {
        let interval = Duration::from_secs(anchor.interval.into());
        let mut watcher = notify::watcher(notify_tx.clone(), interval).expect("couldn't create watch");

        match watcher.watch(anchor.path.clone(), notify::RecursiveMode::Recursive) {
            Err(_) => {
                warn!("unable to set watcher for: '{}'", anchor.path.display());
                continue;
            }
            Ok(_) => {
                // self.watchers.push(watcher); // transfers ownership
                info!("set watcher for: '{}'", anchor.path.display());
            }
        }
    }

    // Set watcher for user drives
    for usr_cfg in &config.users {
        // Client runs for all users
        for anchor in &usr_cfg.1.anchors {
            let interval = Duration::from_secs(anchor.interval.into());
            let mut watcher = notify::watcher(notify_tx.clone(), interval).expect("couldn't create watch");

            match watcher.watch(anchor.path.clone(), notify::RecursiveMode::Recursive) {
                Err(_) => {
                    warn!("unable to set watcher for: '{}'", anchor.path.display());
                    continue;
                }
                Ok(_) => {
                    // self.watchers.push(watcher); // transfers ownership
                    info!("set watcher for: '{}'", anchor.path.display());
                }
            }
        }
    }
}

fn fire_rsync(username: &String, hostname: &String, path: &PathBuf) {
    // rsync -avt my/path/ tony@host:/path/to/stuff
    let src = path;
    let dest = format!("{}@{}:/srv/sinkd/{}", &username, &hostname, &path.display());
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

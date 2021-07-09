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
        error!("sinkd>> couldn't initialize configurations");
        panic!() 
    }
    
    let (notify_tx, notify_rx) = mpsc::channel();
    // let (synch_tx, synch_rx): (mpsc::Sender::<PathBuf>, mpsc::Receiver::<PathBuf>) = mpsc::channel();
    let (synch_tx, synch_rx) = mpsc::channel(); // just pass type for compiler to understand
    // let watchers = Vec::new();   // TODO: needed?     
    set_watchers(&config, notify_tx);

    let watch_thread = thread::spawn(move || {
        watch_entry(notify_rx, synch_tx);
    });

    let synch_thread = thread::spawn(move || {
        synch_entry(&config.sys, &config.users, synch_rx);
    });
}

fn watch_entry(notify_rx: mpsc::Receiver<DebouncedEvent>, synch_tx: mpsc::Sender<PathBuf>) {
    info!("running!");
    loop {
        match notify_rx.recv() {
            Ok(event) => {
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
        last_event: Instant 
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
                    last_event: Instant::now()
                }
            );
        }
        inode_map.insert(name.clone(), inodes);
    }


    loop {

        match synch_rx.try_recv() {
            Ok(path) => {
                let mut found = false;
                for (name, inodes) in inode_map.iter() {
                    for inode in inodes {
                        if path.starts_with(&inode.path) {
                            let elapse = Instant::now().checked_duration_since(inode.last_event).unwrap();
                            if elapse >= inode.interval {
                                let rsync_result = std::process::Command::new("rsync")
                                    .arg("-a") // to archive
                                    // .arg(&path)
                                    .arg(&inode.path)
                                    // TODO check for current user
                                    // rsync -avt my/path/ tony@host:/path/to/stuff
                                    .arg(&sys_cfg.server_addr)
                                    .spawn();
                                match rsync_result {
                                    Err(x) => {
                                        error!("{:?}", x);
                                    }
                                    Ok(_) => {
                                        info!("Called rsync src:{}  ->  dest:{}", 
                                                &path.display(), 
                                                &sys_cfg.server_addr);
                                    }
                                }
                            }
                            

                            found = true;
                            break;
                        }
                    }
                    if found { break; }
                }
            
                // if elapse.ge(Duration::from())
            },
            Err(_) => {
                std::thread::sleep(Duration::from_secs(1));
            }
        }

    }

}

fn set_watchers(config: &Config, notify_tx: mpsc::Sender::<DebouncedEvent>) {

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

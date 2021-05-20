use crate::config::{Config, Anchor};
use notify::{DebouncedEvent, Watcher};
use std::env;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use std::fs;

pub struct Client {
    // multiple produce single consumer
    config: Config,
    events: mpsc::Receiver<notify::DebouncedEvent>, // single rx
    send: mpsc::Sender<notify::DebouncedEvent>,     // clone
    watchers: Vec<notify::RecommendedWatcher>,
}

impl Client {
    pub fn new() -> Client {
        let (tx, rx) = mpsc::channel();
        Client {
            config: Config::new(),
            events: rx,
            send: tx,
            watchers: Vec::new(),
        }
    }

    // infinite loop unless broken by interrupt
    pub fn init(&mut self) {
        if !self.config.init() {
            error!("sinkd>> couldn't initialize configurations");
            panic!()
        }
        self.run();
    }

    pub fn run(&mut self) -> ! {
        self.set_watchers();
        loop {
            match self.events.recv() {
                Ok(event) => {
                    match event {
                        // todo: maybe aggregate the calls under the parent folder, to minimize overhead
                        DebouncedEvent::NoticeWrite(_) => {}  // do nothing
                        DebouncedEvent::NoticeRemove(_) => {} // do nothing for notices
                        DebouncedEvent::Create(path) => self.synchronize(path),
                        DebouncedEvent::Write(path) => self.synchronize(path),
                        DebouncedEvent::Chmod(path) => self.synchronize(path),
                        DebouncedEvent::Remove(path) => self.synchronize(path),
                        DebouncedEvent::Rename(path, _) => self.synchronize(path),
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

    fn synchronize(&mut self, path: PathBuf) {
        //! need to pull excludes from config on loaded path
        //! '/srv/sinkd/user' will have permissions of user (to prevent rsync errors)
        //? RSYNC options to consider
        //~ --copy-links  (included with -a, copies where it points to)
        //~ --delete (must be a whole directory, no wildcards)
        //~ --delete-excluded (also delete excluded files)
        //~ --max-size=SIZE (limit size of transfers)

        let rsync_result = std::process::Command::new("rsync")
            .arg("-a") // to archive
            //   .arg("--exclude=exclude*")
            .arg(&path)
            // TODO check for current user
            //.arg("cerberus:/srv/sinkd/tony")
            .arg(&self.config.sys.server_addr)
            .spawn();
        match rsync_result {
            Err(x) => {
                error!("{:?}", x);
            }
            Ok(_) => {
                info!("Called rsync src:{}  ->  dest:{}", 
                      &path.display(), 
                      &self.config.sys.server_addr);
            }
        }
    }

    fn set_watchers(&mut self) {

        for anchor in self.config.sys.shares.iter() {
            let interval = Duration::from_secs(anchor.interval.into());
            let mut watcher = notify::watcher(self.send.clone(), interval).expect("couldn't create watch");

            match watcher.watch(anchor.path.clone(), notify::RecursiveMode::Recursive) {
                Err(_) => {
                    warn!("unable to set watcher for: '{}'", anchor.path.display());
                    continue;
                }
                Ok(_) => {
                    self.watchers.push(watcher); // transfers ownership
                    info!("set watcher for: '{}'", anchor.path.display());
                }
            }
        }

        for usr_cfg in &self.config.users {
            // Client runs for all users
            for anchor in &usr_cfg.1.anchors {
                let interval = Duration::from_secs(anchor.interval.into());
                let mut watcher = notify::watcher(self.send.clone(), interval).expect("couldn't create watch");

                match watcher.watch(anchor.path.clone(), notify::RecursiveMode::Recursive) {
                    Err(_) => {
                        warn!("unable to set watcher for: '{}'", anchor.path.display());
                        continue;
                    }
                    Ok(_) => {
                        self.watchers.push(watcher); // transfers ownership
                        info!("set watcher for: '{}'", anchor.path.display());
                    }
                }
            }
        }

    }

}

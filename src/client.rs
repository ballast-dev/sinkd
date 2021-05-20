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
        // if !self.load_conf() {
        //     error!("Client did not start, unable to load configuration");
        //     return;
        // }
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
            // current user is used
            //   .arg("cerberus:/srv/sinkd/tony")
            .arg("...pulled from config? ")
            .spawn();
        match rsync_result {
            Err(x) => {
                error!("{:?}", x);
            }
            Ok(_) => {
                info!("Called rsync with source: {:?}", &path);
            }
        }
    }

    // fn load_conf(&mut self) -> bool {
    //     self.config.users.clear();
    //     self.config.anchors.clear();

    //     let mut retval = false;
    //     match fs::read_to_string("/etc/sinkd.conf") {
    //         Err(error) => {
    //             error!("unable to open /etc/sinkd.conf, {}", error);
    //         }
    //         Ok(output) => match toml::from_str(&output) {
    //             Err(error) => {
    //                 error!("couldn't parse '/etc/sinkd.conf' {}", error);
    //             }
    //             Ok(toml_parsed) => {
    //                 self.config = toml_parsed;
    //                 retval = true;
    //             }
    //         },
    //     }
    //     return retval;
    // }

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
    }

    // fn conf_append(
    //     &mut self,
    //     file_to_watch: String,
    //     users: Vec<String>,
    //     interval: u32,
    //     excludes: Vec<String>,
    // ) {
    //     // need to clear the vector, or upon initialization
    //     self.config.anchors.push(Anchor {
    //         path: PathBuf::from(file_to_watch),
    //         users,
    //         interval,
    //         excludes,
    //     });
    //     let new_overlook = toml::to_string_pretty(&self.config);

    //     info!("__conf append__\n{:?}", new_overlook);
    // }

    /**
     * upon edit of config
     * restart the daemon
     *
     * sinkd anchor FOLDER [-i | --interval] SECS
     */
    pub fn anchor(&mut self, mut file_to_watch: String, interval: u32, excludes: Vec<String>) {
        info!("anchoring...");
        if &file_to_watch == "." {
            file_to_watch = env::current_dir().unwrap().to_string_lossy().to_string();
        }
        // self.load_conf(); // not sure if daemon should already be running
        // self.config.anchors.push(Anchor {
        //     path: PathBuf::from(file_to_watch.clone()),
        //     users: Vec::new(), // need to pass empty vec
        //     interval,
        //     excludes,
        // });
        //     let mut watcher =  notify::watcher(self.send.clone(), Duration::from_secs(1)).expect("couldn't create watch");
        //     let result = watcher.watch(watch.path.clone(), notify::RecursiveMode::Recursive);

        //     match result {
        //         Err(_) => {
        //             info!(
        //                 "{:<30} not found, unable to set watcher",
        //                 watch.path.display()
        //             );
        //             continue;
        //         }
        //         Ok(_) => {
        //             self.watchers.push(watcher); // transfers ownership
        //             info!("pushed a Parrot, for this dir => {}", watch.path.display());
        //         }
        //     }
        // }
        // info!("anchor points is this -->{:?}", self.config.anchors);
    }
}

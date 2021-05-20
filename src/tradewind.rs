use std::fs;
use std::env;
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::process::exit as exit;
use std::path::PathBuf;
use crate::rigging::*;


pub struct Caravel {
    // multiple produce single consumer
    config: Config,
    events: std::sync::mpsc::Receiver<notify::DebouncedEvent>, // single rx 
    send: std::sync::mpsc::Sender<notify::DebouncedEvent>, // clone
    parrots: Vec<notify::RecommendedWatcher>,
}

impl Caravel {

    pub fn new() -> Caravel {
        let (tx, rx) = channel();
        Caravel {
            config: Config::new(),
            events: rx,
            send: tx,
            parrots: Vec::new(),
        }
    }

    
    // infinite loop unless broken by interrupt
    pub fn daemon(&mut self) {
        self.load_conf();
        self.set_watchers();
        loop {
            match self.events.recv() {
                Ok(event) => {
                    // handle event
                    info!("{:?}", event);
                },
                Err(e) => println!("watch error: {:?}", e),
            }
            std::thread::sleep(std::time::Duration::from_millis(10))
        }
    }


    fn load_conf(&mut self) -> bool {

        self.config.owner.key.clear();
        self.config.owner.name.clear();
        self.config.users.clear();
        self.config.anchor_points.clear();

        match fs::read_to_string("/etc/sinkd.conf") {
            Err(error) => {
                println!("unable to open file '{}'", error);
                return false;
            }
            Ok(output) => {
                self.config = toml::from_str(&output).expect("couldn't parse toml");
                return true;
            }
        }
    }

    fn set_watchers(&mut self) {
        for watch in self.config.anchor_points.iter() {
            let mut watcher = watcher(self.send.clone(), Duration::from_secs(1)).expect("couldn't create watch");
            let result = watcher.watch(watch.path.clone(), RecursiveMode::Recursive);

            match result {
                Err(_) => {
                    println!("{:<30} not found, unable to set watcher", watch.path.display());
                    continue;
                },
                Ok(_) => {
                    self.parrots.push(watcher); // transfers ownership
                    println!("pushed a Parrot, for this dir => {}", watch.path.display());
                }
            }

        }
    }

    fn conf_append(&mut self, file_to_watch: String, users: Vec<String>, interval: u32, excludes: Vec<String>) {
        let new_watch = Anchorage {
            path: PathBuf::from(file_to_watch),
            users,
            interval,
            excludes,
        };
        // need to clear the vector, or upon initialization
        self.config.anchor_points.push(new_watch);
        let new_overlook = toml::to_string_pretty(&self.config);

        println!("__conf append__\n{:?}", new_overlook);
    }

    /**
     * upon edit of config
     * restart the daemon
     * 
     * sinkd anchor FOLDER [-i | --interval] SECS
     */
    pub fn anchor(&mut self, mut file_to_watch: String, interval: u32, excludes: Vec<String>) {
        println!("anchoring...");
        if &file_to_watch == "." {
            file_to_watch = env::current_dir().unwrap().to_string_lossy().to_string();
        }
        self.load_conf();  // not sure if daemon should already be running
        self.config.anchor_points.push(
            Anchorage {
                path: PathBuf::from(file_to_watch.clone()),
                users: Vec::new(), // need to pass empty vec
                interval,
                excludes,
            }
        );

        for watch in self.config.anchor_points.iter() {
            let mut watcher = watcher(self.send.clone(), Duration::from_secs(1)).expect("couldn't create watch");
            let result = watcher.watch(watch.path.clone(), RecursiveMode::Recursive);

            match result {
                Err(_) => {
                    println!("{:<30} not found, unable to set watcher", watch.path.display());
                    continue;
                },
                Ok(_) => {
                    self.parrots.push(watcher); // transfers ownership
                    println!("pushed a Parrot, for this dir => {}", watch.path.display());
                }
            }

        }
        println!("anchor points is this -->{:?}", self.config.anchor_points);

    }


}
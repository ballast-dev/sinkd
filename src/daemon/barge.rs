/* ---------
* B A R G E
* ---------
*/
// use crate::defs;
use std::fs;
use std::path::PathBuf;
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::process::exit as exit;
use crate::defs::*;

pub struct AnchorPoint {
    // watcher: Vec<notify::INotifyWatcher>,
    // directory to watch
    allowed_users: Vec<String>, // other than owner
    path: PathBuf,
    interval: u32,  // cycle time to check changes
    excludes: Vec<String>, 
    // watches: Vec<Watcher>   // how to instantiate
}

impl AnchorPoint {

    pub fn from(user_name: &str, path: PathBuf, interval: u32, excludes: Vec<String>) -> AnchorPoint {
        let mut users: Vec<String> = Vec::new();
        users.push(user_name.to_ascii_lowercase());
        let anchor_point = AnchorPoint {
            allowed_users: users.clone(),
            path,
            interval,
            excludes,
        };
        return anchor_point;
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    pub fn get_path(&self) -> &PathBuf {
        return &self.path;
    }

    pub fn set_interval(&mut self, interval: u32) {
        self.interval = interval;
    }

    pub fn get_interval(&self) -> u32 {
        return self.interval;
    }

    pub fn add_exclude(&mut self, path: PathBuf) -> bool {
        let added: bool = true;
        if added {
            return true;
        } else {
            return false;
        }
    }

    pub fn add_user(&mut self, user: &str) -> bool {
        return true;
    }

    pub fn rm_user(&mut self, user: &str) -> bool {
        return true;
    }
}

// Command line interface

pub struct Barge {
    deployed: bool,
    overlook: Overlook,
}

impl Barge {

    pub fn new() -> Barge {
        Barge {
            deployed: false,
            overlook: Overlook::new()
        }
    }

    pub fn start() {
        // parse overlook
        // start barge daemon
    }

    // to tell daemon to reparse its configuration file
    pub fn restart() {
        // stop
        // start
    }

    pub fn stop() {
        // stop barge daemon
        // garbage collect?
    }


    // infinite loop unless broken by interrupt
    fn run(&self) -> bool {

        // spawn a thread and return condition
        return true
        // listens to socket, waits for message

        // while (true) {
        //     if message.size > 1 {
        //         text = message_read(message);
        //         match text {
        //             update => parse(),
        //             file_change => sink(),
        //         }
        //     }
        // }
    }


    fn load_conf(&mut self) -> bool {
        // overlook file located in /etc/sinkd.conf

        if !self.deployed { // initialize
            self.overlook.owner.key.clear();
            self.overlook.owner.name.clear();
            self.overlook.users.clear();
            self.overlook.watches.clear();
        }

        let toml_str;
        match fs::read_to_string("/etc/sinkd.conf") {
            Err(error) => {
                println!("unable to open file '{}'", error);
                return false;
            }
            Ok(output) => {
                toml_str = output.clone();
            }
        }

        self.overlook = toml::from_str(&toml_str[..]).expect("couldn't parse toml");
        return true;
        // println!("{:#?}", decoded);

        // for watch in &decoded.watches {
        //     println!("{:?}, {:?}, {:?}, {:?}", watch.path, watch.users, watch.interval, watch.excludes);
        // }
    }


    fn conf_append(&mut self, file_to_watch: String, users: Vec<String>, interval: u32, excludes: Vec<String>) {
        let new_watch = Directory {
            path: file_to_watch,
            users,
            interval,
            excludes,
        };
        // need to clear the vector, or upon initialization
        self.overlook.watches.push(new_watch);
        let new_overlook = toml::to_string_pretty(&self.overlook);

        println!("{:?}", new_overlook);
    }

    /**
     * upon edit of overlookuration restart the daemon
     * 
     * sinkd anchor FOLDER [-i | --interval] SECS
     */
    pub fn anchor(&mut self, file_to_watch: String, interval: u32, excludes: Vec<String>) -> bool {

        self.load_conf();  // not sure if daemon should already be running
        self.overlook.watches.push(
            Directory {
                path: file_to_watch,
                users: Vec::new(), // need to pass empty vec
                interval,
                excludes,
            }
        );

        // restart daemon ???


        // self.overlook.users.push(
        //     User {
        //         name: String::from("new_guy"),
        //         address: String::from("atlantis"),
        //         ssh_key: String::from("some_key"),
        //     }
        // );

        // self.conf_append(file_to_watch, users, interval, excludes);


        // Create a channel to receive the events.
        let (tx, rx) = channel();

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = watcher(tx, Duration::from_secs(1)).expect("couldn't create watch");

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        let result = watcher.watch(file_to_watch, RecursiveMode::Recursive);
        match result {
            Err(_) => {
                println!("path not found, unable to set watcher");
                std::process::exit(1);
            },

            Ok(_) => ()
        }

    // TODO: to spawn in in background
        loop {
            match rx.recv() {
            Ok(event) => println!("{:?}", event),
            Err(e) => println!("watch error: {:?}", e),
            }
        }
    }


}





/************************************
 ************* N O T E S ************
************************************/

// pub fn load_arc(arc: &mut Arc) {
//     /*
//      * the location of arc.json will determine the root directory of arc
//      */
//     if !find_arc(arc) {
//         println!("couldn't find 'arc.json' in this directory or any parent directory");
//         exit(1);
//     }

//     let arcj: ArcJson;
//     let contents: String;

//     let read_result = fs::read_to_string(&arc.path.arc);
//     match read_result {
//         Err(_) => {
//             println!("Couldn't read 'arc.json'");
//             exit(1);
//         },
//         Ok(c) => contents = c,
//     }

//     let result = serde_json::from_str(&contents[..]);
//     match result {
//         Err(_) => {
//             println!("couldn't parse arc.json, is it valid json?");
//             exit(1);
//         }
//         Ok(json) => arcj = json,
//     }

//     arc.remote.name = arcj.remote.name;
//     arc.remote.url  = arcj.remote.url;

//     for (k, v) in arcj.projects {
//         // println!("..... --> {}", k);
//         let mut r = Repo{name: k, deps: v};
//         // println!("loaded arc repo....{}", &r.name);
//         arc.repos.push(r);
//     }
// }
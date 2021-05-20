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
use crate::defs::Config;

pub struct AnchorPoint {
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

// Configuration Holder
struct Overlook {
    owner: String,      // owner of the sinkd
    patrol: Vec<AnchorPoint>,
    keys: Vec<String>, // to hold ssh keys or future type 
}

impl Overlook {
    pub fn new() -> Overlook {
        return Overlook {
            owner: String::new(),
            patrol: Vec::new(),
            keys: Vec::new(),
        }
    }
}


// Command line interface

pub struct Barge {
    deployed: bool,
    overlook: Overlook,
    config: String,
}

impl Barge {

    pub fn new() -> Barge {
        Barge {
            deployed: true,
            overlook: Overlook::new(),
            config: String::from("nada")
        }
    }

    pub fn start() {
        // parse config
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


    fn load_conf() -> bool {
        // config file located in /etc/sinkd.conf

        let mut toml_str = String::new();
        match fs::read_to_string("/etc/sinkd.conf") {
            Err(error) => println!("unable to open file '{}'", error),
            Ok(output) => toml_str = output.clone(),
        }

        let decoded: Config = toml::from_str(&toml_str[..]).expect("couldn't parse toml");
        println!("{:#?}", decoded);

        for watch in &decoded.watches {
            println!("{:?}, {:?}, {:?}, {:?}", watch.path, watch.users, watch.interval, watch.excludes);
        }

        true // did it
    }

    /**
     * upon edit of configuration restart the daemon
     * 
     * sinkd anchor FOLDER [-i | --interval] SECS
     */
    pub fn anchor(&mut self, file_to_watch: &str, interval: u32, excludes: Vec<String>) -> bool {

        let this_user_name = "Johnny Goodboy Tyler";
        self.overlook.patrol.push(
            AnchorPoint::from(this_user_name,
                              PathBuf::from(file_to_watch),
                              interval,
                              excludes)
        );

        // anchor point can either be a file or a folder

        // 1 - open yaml file (/etc/sinkd.conf)
        // 2 - deserialize
        // 3 - append new FOLDER 
        // 4 - write new yaml
        // 5 - restart daemon (harbor)
        Barge::load_conf();


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
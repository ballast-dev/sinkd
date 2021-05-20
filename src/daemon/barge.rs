/* ---------
* B A R G E
* ---------
*/
use std::fs;
use std::env;
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::process::exit as exit;
use std::path::PathBuf;
use crate::defs::*;

// Command line interface
pub struct Parrot {
    pub tx: std::sync::mpsc::Sender<notify::DebouncedEvent>,
    pub rx: std::sync::mpsc::Receiver<notify::DebouncedEvent>,
    pub watcher: notify::RecommendedWatcher,
}

// impl Parrot {
//     pub fn new() -> Parrot {
//         Parrot {
//             tx: std::sync::mpsc::Sender<notify::DebouncedEvent>::new(),
            
//         }
//     }
// }

pub struct Barge {
    deployed: bool,
    overlook: Overlook,
    parrots: Vec<Parrot>,
}

impl Barge {

    pub fn new() -> Barge {
        Barge {
            deployed: false,
            overlook: Overlook::new(),
            parrots: Vec::new(),
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
            self.overlook.anchor_points.clear();
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

        // for watch in &decoded.anchor_points {
        //     println!("{:?}, {:?}, {:?}, {:?}", watch.path, watch.users, watch.interval, watch.excludes);
        // }
    }


    fn conf_append(&mut self, file_to_watch: String, users: Vec<String>, interval: u32, excludes: Vec<String>) {
        let new_watch = AnchorPoint {
            path: PathBuf::from(file_to_watch),
            users,
            interval,
            excludes,
        };
        // need to clear the vector, or upon initialization
        self.overlook.anchor_points.push(new_watch);
        let new_overlook = toml::to_string_pretty(&self.overlook);

        println!("__conf append__\n{:?}", new_overlook);
    }

    /**
     * upon edit of overlook
     * restart the daemon
     * 
     * sinkd anchor FOLDER [-i | --interval] SECS
     */
    pub fn anchor(&mut self, mut file_to_watch: String, interval: u32, excludes: Vec<String>) -> bool {
        
        if &file_to_watch == "." {
            file_to_watch = env::current_dir().unwrap().to_string_lossy().to_string();
        }
        self.load_conf();  // not sure if daemon should already be running
        self.overlook.anchor_points.push(
            AnchorPoint {
                path: PathBuf::from(file_to_watch.clone()),
                users: Vec::new(), // need to pass empty vec
                interval,
                excludes,
            }
        );

        // restart daemon ???


        // Create a channel to receive the events.

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.

        // std::sync::mpsc::Sender<notify::DebouncedEvent>
        // std::sync::mpsc::Receiver<notify::DebouncedEvent>
        for watch in self.overlook.anchor_points.iter() {
            let (tx, rx) = channel();
            let mut watcher = watcher(tx, Duration::from_secs(1)).expect("couldn't create watch");
            let result = watcher.watch(watch.path.clone(), RecursiveMode::Recursive);

            match result {
                Err(_) => {
                    println!("{:<30} not found, unable to set watcher", watch.path.display());
                    continue;
                },

                Ok(_) => ()
            }

            // self.parrots.push( Parrot {
            //         tx,  
            //         rx, 
            //         watcher: watcher.clone() 
            //     } 
            // );
            println!("pushed a Parrot, for this dir => {}", watch.path.display());

        }
        println!("anchor points is this -->{:?}", self.overlook.anchor_points);


        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.

    // TODO: to spawn in in background
        loop {
            for parrot in self.parrots.iter() {
                match parrot.rx.recv() {
                    Ok(event) => println!("{:?}", event),
                    Err(e) => println!("watch error: {:?}", e),
                }
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
/**
 * B A R G E 
 *
 * --- 
 *
 * Client side of sinkd
 * will hook into anchor and fail if anchor is not there
 * 
 * be able to start and stop daemon 
 * this binary will be invoked in usr directory i.e. /usr/local/bin
 */
extern crate notify;
extern crate yaml_rust;

use notify::{Watcher, RecursiveMode, watcher};
use yaml_rust::{YamlLoader, YamlEmitter};

use std::sync::mpsc::channel;
use std::time::Duration;
use std::env;
use std::path::PathBuf;

pub struct AnchorPoint {
    // directory to watch
    path: PathBuf,
    interval: u32,  // cycle time to check changes
}

impl AnchorPoint {

    pub fn from(path: &str, interval: u32) -> AnchorPoint {
        let path_buf = PathBuf::from(path);
        return AnchorPoint {
            path: path_buf,
            interval, 
        }
    }

    pub fn set_interval(&mut self, interval: u32) {
        self.interval = interval;
    }
}



pub struct Barge {
    anchor_points: Vec<AnchorPoint>,
}

impl Barge {

    pub fn load_config() -> bool {
        // config should always be located in /etc/sinkd.conf
        // fs::read
        return true;
    }

    /**
     * upon edit of configuration restart the daemon
     */
    pub fn watch(&mut self, file_to_watch: &str, interval: u32) -> bool {

        self.anchor_points.push(AnchorPoint::from(file_to_watch, interval));
        // anchor point can either be a file or a folder

        // need to start daemon from config file
        // need to parse json
        // set up json within /etc/sinkd.json


        // serde already does json

        // Create a channel to receive the events.
        let (tx, rx) = channel();

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

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


    // to tell daemon to reparse its configuration file
    pub fn update_daemon() {
        // parse config file
    }

    // infinite loop unless broken by interrupt
    pub fn run_daemon() {
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

    pub fn parse_conf() -> bool {
        // config file located in /etc/sinkd.conf

    //     let s =
    // "
    // foo:
    //     - list1
    //     - list2
    // bar:
    //     - 1
    //     - 2.0
    // ";
    //     let docs = YamlLoader::load_from_str(s).unwrap();

    //     // Multi document support, doc is a yaml::Yaml
    //     let doc = &docs[0];

    //     // Debug support
    //     println!("{:?}", doc);

    //     // Index access for map & array
    //     assert_eq!(doc["foo"][0].as_str().unwrap(), "list1");
    //     assert_eq!(doc["bar"][1].as_f64().unwrap(), 2.0);

    //     // Chained key/array access is checked and won't panic,
    //     // return BadValue if they are not exist.
    //     assert!(doc["INVALID_KEY"][100].is_badvalue());

        // // Dump the YAML object
        // let mut out_str = String::new();
        // {
        //     let mut emitter = YamlEmitter::new(&mut out_str);
        //     emitter.dump(doc).unwrap(); // dump the YAML object to a String
        // }
        // println!("{}", out_str);
    true
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

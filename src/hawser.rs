extern crate log;

use log::{error, info, warn, Record, Level, Metadata, LevelFilter};
use std::fs::OpenOptions;

static LOGGER: Hawser = Hawser::init();

struct Hawser {
    log_path: std::path::PathBuf,
}  // big rope to moor ship to harbor

impl Hawser {
    const fn init() -> Self {
        // setup logger
        log::set_logger(&LOGGER).unwrap();
        log::set_max_level(LevelFilter::Info);
        Hawser { log_path: get_sinkd_path() }
    }
}

// implement trait 
impl log::Log for Hawser {

    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // let file = OpenOptions::new().append(true).create(true).open("foo.txt");
            println!("{} - {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}

// fn main() {
//     println!("Hello, world!");    
//     info!("hello log");
//     warn!("warning");
//     error!("oops");
// }



pub fn get_sinkd_path() -> std::path::PathBuf {
    let user = env!("USER");
    let sinkd_path = if cfg!(target_os = "macos") {
        std::path::Path::new("/Users").join(user).join(".sinkd")
    } else {
        std::path::Path::new("/home").join(user).join(".sinkd")
    };
    
    if !sinkd_path.exists() {
        match std::fs::create_dir(&sinkd_path) {
            Err(why) => println!("cannot create {:?}, {:?}", sinkd_path, why.kind()),
            Ok(_) => {},
        }
    }
    return sinkd_path;
} 
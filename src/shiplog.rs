extern crate log;

use log::{Record, Level, Metadata, LevelFilter};
use std::fs::OpenOptions;
use std::io::prelude::*;  // for writeln!
use crate::utils;


pub struct ShipLog { // big rope to moor ship to harbor
    file: std::fs::File
} 

impl ShipLog {
    pub fn new() -> Self {
        ShipLog {
            file: OpenOptions::new()
                .append(true)
                .create(true)
                .open("/var/log/sinkd.log")
                .expect("couldn't create log file")
        }
    }

    pub fn init() {
        log::set_boxed_logger(Box::new(ShipLog::new())).unwrap();
        log::set_max_level(LevelFilter::Info);
    }
    
}

impl log::Log for ShipLog {

    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            writeln!(&self.file, "{}[{}]{}",
                     utils::get_timestamp("%T"), 
                     record.level(), 
                     record.args()).expect("couldn't write to log file");
        }
    }

    fn flush(&self) {}
}

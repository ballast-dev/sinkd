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
                .open(utils::LOG_PATH)
                .expect("couldn't create log file")
        }
    }

    pub fn init() {
        log::set_boxed_logger(Box::new(ShipLog::new())).unwrap();
        log::set_max_level(LevelFilter::Info);
    }
    
    fn log_rotate(&self) -> bool {
        // std::mem::drop
        // how to close the file to rotate? 
        // drop(self.file);
        return true;
    }
}

impl log::Log for ShipLog {

    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let ten_megabytes: u64 = 1024 * 10000;
            let file_size = &self.file.metadata().unwrap().len();
            if file_size > &ten_megabytes {
                writeln!(&self.file, "{}[{}]FILESIZE OVER TEN-MEGABYTES({}): {}",
                        utils::get_timestamp("%T"), 
                        record.level(), 
                        &self.file.metadata().unwrap().len(),
                        record.args()).expect("couldn't write to log file");
                self.log_rotate();
            } else {
                writeln!(&self.file, "{}[{}]-{}",
                         utils::get_timestamp("%T"), 
                         record.level(), 
                         record.args()).expect("couldn't write to log file");
            }
        }
    }

    fn flush(&self) {}
}

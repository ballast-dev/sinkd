extern crate log;

use log::{Record, Level, Metadata, LevelFilter};
use std::fs::OpenOptions;
use std::io::prelude::*;  // for writeln!
use crate::utils;

const TEN_MEGABYTES: u64 = 1024^2 * 10;

pub struct ShipLog { // big rope to moor ship to harbor
    file: std::fs::File,
} 

impl ShipLog {
    pub fn new() -> Self {
        ShipLog {
            file: OpenOptions::new()
                .append(true)
                .create(true)
                .open(utils::LOG_PATH)
                .expect("couldn't create log file"),
        }
    }

    pub fn init() {
        log::set_boxed_logger(Box::new(ShipLog::new())).unwrap();
        log::set_max_level(LevelFilter::Debug);
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
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // let file_size = self.file.metadata().unwrap().len();
            // if file_size < TEN_MEGABYTES {
            // }
            
            writeln!(&self.file, "{}[{}]-{}",
            utils::get_timestamp("%T"), 
            record.level(), 
            record.args()).expect("couldn't write to log file");


            // writeln!(&self.file, "{}[{}]FILESIZE OVER TEN-MEGABYTES({}): {}",
            //         utils::get_timestamp("%T"), 
            //         record.level(), 
            //         &self.file.metadata().unwrap().len(),
            //         record.args()).expect("couldn't write to log file");
        }
    }

    fn flush(&self) {}
}

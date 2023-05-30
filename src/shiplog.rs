extern crate log;

use crate::{
    outcome::Outcome,
    utils::{self, Parameters}
};
use log::{Level, LevelFilter, Metadata, Record};
use std::fs::OpenOptions;
use std::io::prelude::*; // for writeln!

const TEN_MEGABYTES: u64 = (1024 ^ 2) * 10;

pub struct ShipLog {
    // big rope to moor ship to harbor
    file: std::fs::File,
}

impl ShipLog {
    pub fn new(params: &Parameters) -> Self {
        ShipLog {
            file: OpenOptions::new()
                .append(true)
                .create(true)
                .open(params.get_log_path())
                .expect("couldn't create log file"),
        }
    }

    pub fn init(params: &Parameters) {
        log::set_boxed_logger(Box::new(ShipLog::new(params))).unwrap();
        log::set_max_level(LevelFilter::Debug);
        println!("Logging to: '{}'", params.get_log_path().display());
    }

    fn log_rotate(&self) -> bool {
        // std::mem::drop
        // how to close the file to rotate?
        // drop(self.file);
        true
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

            writeln!(
                &self.file,
                "{}[{}]-{}",
                utils::get_timestamp("%T"),
                record.level(),
                record.args()
            )
            .expect("couldn't write to log file");

            // writeln!(&self.file, "{}[{}]FILESIZE OVER TEN-MEGABYTES({}): {}",
            //         utils::get_timestamp("%T"),
            //         record.level(),
            //         &self.file.metadata().unwrap().len(),
            //         record.args()).expect("couldn't write to log file");
        }
    }

    fn flush(&self) {}
}

pub fn init(params: &Parameters) -> Outcome<()> {
    // if params.debug_mode {
    //     std::fs::create_dir_all("~/.sinkd").unwrap();
    // }
    match utils::create_log_file(params) {
        Err(e) => Err(e),
        Ok(_) => {
            ShipLog::init(params);
            info!("log initialized");
            match utils::create_pid_file(params) {
                Err(e) => Err(e),
                Ok(_) => Ok(()),
            }
        }
    }
}

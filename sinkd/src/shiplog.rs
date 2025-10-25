use log::{Level, LevelFilter, Metadata, Record};
use std::fs::OpenOptions;
use std::io::prelude::*;

use crate::{config, outcome::Outcome, parameters::Parameters, time};

//const TEN_MEGABYTES: u64 = (1024 ^ 2) * 10;

pub struct ShipLog {
    file: std::fs::File,
    debug_level: u8,
}

impl ShipLog {
    fn new(params: &Parameters) -> Self {
        ShipLog {
            file: OpenOptions::new()
                .write(true)
                .append(params.debug == 0)
                .truncate(params.debug > 0)
                .create(true)
                .open(&params.log_path)
                .expect("couldn't create log file"),
            debug_level: params.debug,
        }
    }

    //fn log_rotate(mut self, path: PathBuf) {
    //    self.file.flush().expect("unable to flush log file");
    //    drop(self.file); // drop closes the file
    //    self.file = OpenOptions::new()
    //        .append(true)
    //        .create(true)
    //        .open(path)
    //        .expect("couldn't create log file")
    //}
}

impl log::Log for ShipLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let target = record.target();
            if target.starts_with("paho") {
                if self.debug_level == 2 {
                    writeln!(
                        &self.file,
                        "{}[MQTT][{}]-{}",
                        time::stamp(None),
                        record.level(),
                        record.args()
                    )
                    .expect("couldn't write to log file");
                }
            } else {
                // let file_size = self.file.metadata().unwrap().len();
                // if file_size < TEN_MEGABYTES {
                // }

                writeln!(
                    &self.file,
                    "{}[{}]-{}",
                    time::stamp(None),
                    record.level(),
                    record.args()
                )
                .expect("couldn't write to log file");

                // writeln!(&self.file, "{}[{}]FILESIZE OVER TEN-MEGABYTES({}): {}",
                //         config::get_timestamp("%T"),
                //         record.level(),
                //         &self.file.metadata().unwrap().len(),
                //         record.args()).expect("couldn't write to log file");
            }
        }
    }

    fn flush(&self) {}
}

pub fn init(params: &Parameters) -> Outcome<()> {
    create_log_file(params)?;
    log::set_boxed_logger(Box::new(ShipLog::new(params))).expect("unable to create logger");
    log::set_max_level(match params.verbosity {
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        _ => LevelFilter::Debug,
        // _ => LevelFilter::Trace,
    });
    println!("Logging to: '{}'", params.log_path.display());
    info!("======== ⚓ log initialized ⚓ ========");
    Ok(())
}

fn create_log_file(params: &Parameters) -> Outcome<()> {
    if params.debug == 0 && !config::have_permissions() {
        return bad!("Need to be root to create log file");
    }

    if !params.log_path.exists() && params.debug > 0 {
        if let Err(why) = std::fs::File::create(&params.log_path) {
            return bad!(
                "cannot create '{}' {}",
                params.log_path.display(),
                why.kind()
            );
        }
    }
    Ok(()) // already created
}

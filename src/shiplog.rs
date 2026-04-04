use log::{info, Level, LevelFilter, Metadata, Record};
use std::fs::OpenOptions;
use std::io::prelude::*;

use crate::{config, outcome::Outcome, parameters::SharedDaemonParams, time};

pub struct ShipLog {
    file: std::fs::File,
    debug_level: u8,
}

impl ShipLog {
    fn new(shared: &SharedDaemonParams) -> Self {
        ShipLog {
            file: OpenOptions::new()
                .write(true)
                .append(shared.debug == 0)
                .truncate(shared.debug > 0)
                .create(true)
                .open(&shared.log_path)
                .expect("couldn't create log file"),
            debug_level: shared.debug,
        }
    }
}

impl log::Log for ShipLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let target = record.target();
            let third_party_ipc = target.starts_with("zenoh") || target.starts_with("paho");
            if third_party_ipc && self.debug_level != 2 {
                return;
            }
            if third_party_ipc && self.debug_level == 2 {
                writeln!(
                    &self.file,
                    "{}[IPC][{}]-{}",
                    time::stamp(None),
                    record.level(),
                    record.args()
                )
                .expect("couldn't write to log file");
            } else if !third_party_ipc {
                writeln!(
                    &self.file,
                    "{}[{}]-{}",
                    time::stamp(None),
                    record.level(),
                    record.args()
                )
                .expect("couldn't write to log file");
            }
        }
    }

    fn flush(&self) {}
}

pub fn init(shared: &SharedDaemonParams) -> Outcome<()> {
    create_log_file(shared)?;
    log::set_boxed_logger(Box::new(ShipLog::new(shared)))
        .map_err(|e| format!("unable to create logger: {e}"))?;
    log::set_max_level(match shared.verbosity {
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    });
    println!("Logging to: '{}'", shared.log_path.display());
    info!("======== ⚓ log initialized ⚓ ========");
    Ok(())
}

fn create_log_file(shared: &SharedDaemonParams) -> Outcome<()> {
    if shared.debug == 0 && !config::have_permissions() {
        return bad!("Need to be root to create log file");
    }

    if !shared.log_path.exists() && shared.debug > 0 {
        if let Err(why) = std::fs::File::create(&shared.log_path) {
            return bad!(
                "cannot create '{}' {}",
                shared.log_path.display(),
                why.kind()
            );
        }
    }
    Ok(())
}

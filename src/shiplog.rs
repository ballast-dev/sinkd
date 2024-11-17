use libc::{c_char, c_uint};
use log::{Level, LevelFilter, Metadata, Record};
use std::io::prelude::*;
use std::{
    ffi::{CStr, CString},
    fs::OpenOptions, // for writeln!
                     //path::PathBuf,
                     //time::{Duration, Instant},
};

use crate::{config, outcome::Outcome, parameters::Parameters};

//const TEN_MEGABYTES: u64 = (1024 ^ 2) * 10;

#[link(name = "timestamp", kind = "static")]
extern "C" {
    fn timestamp(ret_str: *mut c_char, size: c_uint, fmt_str: *const c_char);
}

pub fn get_timestamp(fmt_str: &str) -> String {
    const TIMESTAMP_LENGTH: usize = 25;
    let mut buffer = vec![0u8; TIMESTAMP_LENGTH];

    let ret_ptr = buffer.as_mut_ptr().cast::<c_char>();
    let c_fmt_str = CString::new(fmt_str.as_bytes()).expect("failed to create CString");

    unsafe {
        timestamp(ret_ptr, TIMESTAMP_LENGTH as c_uint, c_fmt_str.as_ptr());
    }

    // convert buffer to CStr
    let c_str = unsafe { CStr::from_ptr(ret_ptr) };
    // convert CStr to Rust String
    c_str.to_string_lossy().into_owned()
}

pub struct ShipLog {
    file: std::fs::File,
    debug_level: u8,
}

impl ShipLog {
    fn new(params: &Parameters) -> Self {
        ShipLog {
            file: OpenOptions::new()
                .append(true)
                .create(true)
                .open(&params.log_path)
                .expect("couldn't create log file"),
            debug_level: params.debug,
        }
    }

    pub fn init(params: &Parameters) -> Outcome<()> {
        create_log_file(params)?;
        log::set_boxed_logger(Box::new(Self::new(params))).expect("unable to create logger");
        log::set_max_level(match params.verbosity {
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            _ => LevelFilter::Debug,
            // _ => LevelFilter::Trace,
        });
        println!("Logging to: '{}'", params.log_path.display());
        info!("log initialized");
        Ok(())
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
                        get_timestamp("%T"),
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
                    get_timestamp("%T"),
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

fn create_pid_file(params: &Parameters) -> Outcome<()> {
    if params.debug == 0 && !config::have_permissions() {
        return bad!("need to be root");
    }
    if !params.pid_path.exists() {
        // match std::fs::create_dir_all(&pid_file) {
        info!("creating pid file: {}", params.pid_path.display());
        if let Err(why) = std::fs::File::create(&params.pid_path) {
            error!(
                "cannot create '{}' {}",
                params.pid_path.display(),
                why.kind()
            );
            return bad!(
                "cannot create '{}' {}",
                params.pid_path.display(),
                why.kind()
            );
        }
    }
    Ok(())
    // already created
    // fs::File::create(PID_FILE).expect("unable to create pid file, permissions?");
    // let metadata = pid_file.metadata().unwrap();
    // let mut permissions = metadata.permissions();
    // permissions.set_readonly(false);
    // fs::set_permissions(&pid_path, permissions).expect("cannot set permission");
}

fn create_log_file(params: &Parameters) -> Outcome<()> {
    if params.debug == 0 && !config::have_permissions() {
        return bad!("Need to be root to create log file");
    }

    if !params.log_path.exists() && params.debug >= 1 {
        if let Err(why) = std::fs::File::create(&params.log_path) {
            // truncates file if exists
            return bad!(
                "cannot create '{}' {}",
                params.log_path.display(),
                why.kind()
            );
        }
    }
    Ok(()) // already created
}

pub fn get_pid(params: &Parameters) -> Outcome<u32> {
    if !params.pid_path.exists() {
        bad!("pid file not found")
    } else {
        match std::fs::read(&params.pid_path) {
            Err(err) => {
                bad!(format!(
                    "Cannot read {}: {}",
                    params.pid_path.display(),
                    err
                ))
            }
            Ok(contents) => {
                let pid_str = String::from_utf8_lossy(&contents);
                match pid_str.parse::<u32>() {
                    Err(e) => {
                        bad!("Couldn't parse pid: {}", e)
                    }
                    Ok(pid) => Ok(pid),
                }
            }
        }
    }
}

pub fn set_pid(params: &Parameters, pid: u32) -> Outcome<()> {
    if !params.pid_path.exists() {
        create_pid_file(params)?;
    }
    if pid == 0 {
        // if Parent process
        unsafe {
            // pid_file is typically set so unwrap here is safe
            let c_str = CString::new(params.pid_path.to_str().unwrap()).unwrap();
            // delete a name and possibly the file it refers to
            libc::unlink(c_str.into_raw());
        }
    } else if let Err(e) = std::fs::write(&params.pid_path, pid.to_string()) {
        return bad!("couldn't write to '{}' {}", &params.pid_path.display(), e);
    }
    Ok(())
}

pub fn rm_pid(params: &Parameters) -> Outcome<()> {
    if params.debug == 0 && !config::have_permissions() {
        return bad!("Need to be root to create pid file");
    }
    if params.pid_path.exists() {
        std::fs::remove_file(&params.pid_path)?;
    }
    Ok(())
}

// Common Utilities
use libc::{c_char, c_uint};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{fancy, ipc, outcome::Outcome};

const TIMESTAMP_LENGTH: u8 = 25;

// TODO: move this into section of /etc/sinkd.conf
pub struct Parameters <'a>{
    pub verbosity: u8,
    pub clear_logs: bool,
    pub debug_mode: bool,
    pub log_path: Arc<&'a Path>,
    pub pid_path: Arc<&'a Path>,
    pub system_config: Arc<PathBuf>,
    pub user_configs: Arc<Vec<PathBuf>>,
}

impl <'a> Parameters <'a>{
    pub fn new(
        verbosity: u8, 
        debug: bool, 
        system_config: PathBuf, 
        user_configs: Option<Vec<PathBuf>>
    ) -> Self {
        Parameters {
            verbosity: if debug { 4 } else { verbosity },
            clear_logs: if debug { true } else { false },
            debug_mode: debug,
            log_path: if debug {
                Arc::new(Path::new("/tmp/sinkd.log"))
            } else {
                Arc::new(Path::new("/var/log/sinkd.log")) 
            },
            pid_path: if debug {
                Arc::new(Path::new("/tmp/sinkd.pid"))
            } else {
                Arc::new(Path::new("/run/sinkd.pid"))
            },
            system_config: Arc::new(system_config),
            user_configs: if let Some(_cfgs) = user_configs {
                Arc::new(_cfgs)
            } else {
                Arc::new(vec![PathBuf::from("~/.config/sinkd.conf")]) 
            }
        }
    }
}

#[link(name = "timestamp", kind = "static")]
extern "C" {
    fn timestamp(ret_str: *mut c_char, size: c_uint, fmt_str: *const c_char);
}

pub fn get_timestamp(fmt_str: &str) -> String {
    let ret_str = CString::new(Vec::with_capacity(TIMESTAMP_LENGTH.into())).unwrap();
    let ret_ptr: *mut c_char = ret_str.into_raw();

    let _fmt_str = CString::new(fmt_str.as_bytes()).unwrap();
    let stamp: CString;
    unsafe {
        timestamp(ret_ptr, TIMESTAMP_LENGTH.into(), _fmt_str.as_ptr());
        stamp = CString::from_raw(ret_ptr);
    }
    let v = stamp.into_bytes();
    String::from_utf8_lossy(&v).into_owned()
}

pub fn have_permissions() -> bool {
    unsafe {
        // get effective user id
        libc::geteuid() == 0
    }
}

pub fn create_pid_file(params: &Parameters) -> Outcome<()> {
    if !params.debug_mode && !have_permissions() {
        return bad!("need to be root");
    }
    if !params.pid_path.exists() {
        // match std::fs::create_dir_all(&pid_file) {
        match std::fs::File::create(*params.pid_path) {
            Err(why) => {
                bad!("cannot create {:?}, {:?}", params.pid_path, why.kind())
            }
            Ok(_) => Ok(()),
        }
    } else {
        Ok(()) // already created
    }
    // fs::File::create(PID_FILE).expect("unable to create pid file, permissions?");
    // let metadata = pid_file.metadata().unwrap();
    // let mut permissions = metadata.permissions();
    // permissions.set_readonly(false);
    // fs::set_permissions(&pid_path, permissions).expect("cannot set permission");
}

pub fn create_log_file(params: &Parameters) -> Outcome<()> {
    if !params.debug_mode && !have_permissions() {
        return bad!("Need to be root to create log file");
    }

    if !params.log_path.exists() || params.clear_logs {
        if let Err(why) = std::fs::File::create(*params.log_path) {
            // truncates file if exists
            return bad!("cannot create {:?}, {:?}", params.log_path, why.kind());
        }
    }
    Ok(()) // already created
}

pub fn get_pid(params: &Parameters) -> Outcome<u16> {
    // let user = env!("USER");
    // let sinkd_path = if cfg!(target_os = "macos") {
    //     path::Path::new("/Users").join(user).join(".sinkd")
    // } else {
    //     path::Path::new("/home").join(user).join(".sinkd")
    // };

    if !params.pid_path.exists() {
        bad!("pid file not found")
    } else {
        match std::fs::read(*params.pid_path) {
            Err(err) => {
                bad!(format!("Cannot read {}: {}", params.pid_path.display(), err))
            }
            Ok(contents) => {
                let pid_str = String::from_utf8_lossy(&contents);
                match pid_str.parse::<u16>() {
                    Err(e2) => {
                        // err_msg!("Couldn't parse pid: {}", e2)
                        bad!("oh yeah baby!")
                    }
                    Ok(pid) => Ok(pid),
                }
            }
        }
    }
}

pub fn set_pid(params: &Parameters, pid: u16) -> Result<(), String> {
    if !params.pid_path.exists() {
        return Err(String::from("pid file not found"));
    }

    if pid == 0 {
        unsafe {
            // pid_file is typically set so unwrap here is safe
            let c_str = CString::new(params.pid_path.to_str().unwrap()).unwrap();
            libc::unlink(c_str.into_raw());
        }
        Ok(())
    } else {
        match std::fs::write(*params.pid_path, pid.to_ne_bytes()) {
            Err(err) => {
                let err_str = format!("couldn't clear pid in ~/.sinkd/pid\n{}", err);
                Err(err_str)
            }
            Ok(()) => Ok(()),
        }
    }
}

/// Both macOS and Linux have the uname command
pub fn get_hostname() -> String {
    match std::process::Command::new("uname").arg("-n").output() {
        Err(e) => {
            error!("uname didn't work? {}", e);
            String::from("uname-error")
        }
        Ok(output) => {
            let mut v = output.stdout.to_ascii_lowercase();
            v.truncate(v.len() - 1); // strip newline
            debug!("{}", std::str::from_utf8(&v).unwrap());
            String::from_utf8(v).unwrap_or_else(|_| {
                error!("invalid string from uname -a");
                String::from("invalid-hostname")
            })
        }
    }
}

/// Both macOS and Linux have the whoami command
pub fn get_username() -> String {
    match std::process::Command::new("whoami").output() {
        Err(e) => {
            error!("whoami didn't work? {}", e);
            String::from("whoami error")
        }
        Ok(output) => {
            let mut v = output.stdout.to_ascii_lowercase();
            v.truncate(v.len() - 1); // strip newline
            debug!("{}", std::str::from_utf8(&v).unwrap());
            String::from_utf8(v).unwrap_or_else(|_| {
                error!("invalid string from whoami");
                String::from("invalid-username")
            })
        }
    }
}

// this will resolve all known paths, converts relative to absolute 
pub fn resolve(path: &str) -> Outcome<PathBuf> {
    let mut resolved_path = PathBuf::from(path);
    if path.starts_with("~") {
        let home = match std::env::var("HOME") {
            Ok(home_dir) => home_dir,
            Err(e) => {
                return bad!("cannot resolve user path: {}", e);
            }
        };
        resolved_path = Path::new(&resolved_path.strip_prefix("~").unwrap()).to_path_buf();
        resolved_path = Path::new(&home).join(resolved_path);
        match resolved_path.canonicalize() {
            Ok(normalized) => Ok(normalized),
            Err(e) => bad!("cannot canonicalize: '{}' {}", resolved_path.display(), e)
        }
    } else {
        match resolved_path.canonicalize() {
            Ok(normalized) => Ok(normalized),
            Err(e) => bad!("cannot canonicalize: '{}' {}", resolved_path.display(), e)
        }
    }
}

pub fn rsync(payload: &ipc::Payload) {
    // Agnostic pathing allows sinkd not to care about user folder structure

    debug!("{}", payload);

    // need to account for shared folders
    // and local sync? maybe useful for testing
    let mut cmd = std::process::Command::new("rsync"); // have to bind at .new()
    cmd.arg("-atR") // archive, timestamps, relative
        .arg("--delete") // delete on destination if not reflected in source
        //? RSYNC options to consider
        // .arg("--delete-excluded")
        // .arg("--max-size=SIZE") // (limit size of transfers)
        // .arg("--exclude=PATTERN") // loop through to all all paths
        ;

    if &payload.dest == "client" {
        // TODO: hostname + username for full path
        cmd.args(&payload.paths).arg(&payload.hostname);
    } else {
        cmd.arg(&payload.hostname).args(&payload.paths);
    }

    match cmd.spawn() {
        Err(x) => {
            error!("{:?}", x);
        }
        Ok(_) => {
            info!("called rsync! dest:{} paths:", &payload.dest);
            for path in &payload.paths {
                info!("\t{}", path.display());
            }
        }
    }
}
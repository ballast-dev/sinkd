use clap::parser::ValuesRef;
// Common Utilities
use libc::{c_char, c_uint};
use std::{
    ffi::CString,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{fancy, ipc, outcome::Outcome};

const TIMESTAMP_LENGTH: u8 = 25;

// TODO: move this into section of /etc/sinkd.conf
pub struct Parameters<'a> {
    pub verbosity: u8,
    pub clear_logs: bool,
    pub debug: bool,
    pub log_path: Arc<&'a Path>,
    pub pid_path: Arc<&'a Path>,
    pub system_config: Arc<PathBuf>,
    pub user_configs: Arc<Vec<PathBuf>>,
}

impl<'a> Parameters<'a> {
    pub fn new(
        verbosity: u8,
        debug: bool,
        system_config: &String,
        user_configs: Option<ValuesRef<String>>,
    ) -> Outcome<Self> {
        Ok(Parameters {
            verbosity: match (debug, verbosity) {
                (true, _) => 4,
                (false, 0) => 2, // default to warn log level
                (_, v) => v,
            },
            clear_logs: if debug { true } else { false },
            debug,
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
            system_config: Arc::new(Self::resolve_system_config(system_config)?),
            user_configs: Arc::new(Self::load_user_configs(user_configs)?),
        })
    }

    fn resolve_system_config(system_config: &String) -> Outcome<PathBuf> {
        match resolve(system_config) {
            Ok(normalized) => {
                if normalized.is_dir() {
                    // TODO: have error codes
                    bad!(
                        "{} is a directory not a file, aborting",
                        normalized.display()
                    )
                } else {
                    Ok(normalized)
                }
            }
            Err(e) => bad!("system config path error: {}", e),
        }
    }

    fn load_user_configs(user_configs: Option<ValuesRef<String>>) -> Outcome<Vec<PathBuf>> {
        match user_configs {
            Some(cfgs) => Ok(cfgs.map(|p| PathBuf::from(p)).collect()),
            None => Ok(vec![]) // server doesn't need user configs
        }
    }

    // this needs to resolve on "start --client"
    pub fn resolve_user_configs(&mut self) -> Outcome<bool> {
        let mut resolved_configs = Vec::new();

        for cfg in &*self.user_configs {
            let path = cfg.as_path().to_str().unwrap();
            let normalized = resolve(path)?;
            if normalized.is_dir() {
                return bad!(
                    "{} is a directory, not a file; aborting",
                    normalized.display()
                );
            }
            resolved_configs.push(normalized);
        }

        // Replace the old Arc with a new one containing the updated configs
        self.user_configs = Arc::new(resolved_configs);

        Ok(true)
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
    if !params.debug && !have_permissions() {
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
    if !params.debug && !have_permissions() {
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
                bad!(format!(
                    "Cannot read {}: {}",
                    params.pid_path.display(),
                    err
                ))
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
    // NOTE: `~` is a shell expansion not handled by system calls 
    if path.starts_with("~/") {
        let mut p = match std::env::var("HOME") {
            Ok(home_dir) => PathBuf::from(home_dir),
            Err(e) => {
                return bad!("HOME env var not defined: {}", e);
            }
        };
        p.push(&path.strip_prefix("~/").unwrap());
        match p.canonicalize() {
            Ok(resolved) => Ok(resolved),
            Err(e) => bad!("cannot canonicalize: '{}' {}", p.display(), e),
        }
    } else {
        match PathBuf::from(path).canonicalize() {
            Ok(resolved) => Ok(resolved),
            Err(e) => bad!("cannot canonicalize: '{}' {}", path, e),
        }
    }
}

//? This command will not spawn new instances
//? if mosquitto already active.
pub fn start_mosquitto() -> Outcome<()> {
    debug!(">> spawn mosquitto daemon");
    if let Err(spawn_error) = std::process::Command::new("mosquitto").arg("-d").spawn() {
        return bad!(format!(
            "Is mosquitto installed and in path? >> {}",
            spawn_error
        ));
    }
    Ok(())
}

pub fn daemon(
    func: fn(&Parameters) -> Outcome<()>,
    app_type: &str,
    params: &Parameters,
) -> Outcome<()> {
    use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
    use nix::unistd::{fork, ForkResult};

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            let start_time = Instant::now();
            let timeout = Duration::from_secs(2);

            while start_time.elapsed() < timeout {
                match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                    Ok(status) => match status {
                        WaitStatus::Exited(_, _) => {
                            return bad!(format!("{} encountered error", app_type))
                        }
                        _ => (),
                    },
                    Err(e) => eprintln!("Failed to wait on child?: {}", e),
                }
                std::thread::sleep(Duration::from_secs(1));
            }
            println!("spawned, logging to '{}'", params.log_path.display());
            Ok(())
        }
        Ok(ForkResult::Child) => {
            info!("about to start daemon...");
            func(params)
        }
        Err(_) => {
            bad!("Failed to fork process")
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

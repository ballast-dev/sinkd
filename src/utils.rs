use clap::parser::ValuesRef;
// Common Utilities
use libc::{c_char, c_uint};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::{
    ffi::CString,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{fancy, ipc, outcome::Outcome};

const TIMESTAMP_LENGTH: u8 = 25;

#[derive(PartialEq)]
pub enum DaemonType {
    Client,
    Server,
}

// TODO: move this into section of /etc/sinkd.conf
pub struct Parameters<'a> {
    pub daemon_type: &'a DaemonType,
    pub verbosity: u8,
    pub clear_logs: bool,
    pub debug: bool,
    pub log_path: Arc<&'a Path>,
    pub pid_path: Arc<&'a Path>,
    pub system_config: Arc<PathBuf>,
    pub user_configs: Arc<Vec<PathBuf>>,
}

impl<'a> std::fmt::Display for Parameters<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let deamon_type: &str = if *self.daemon_type == DaemonType::Client {
            "daemon:type:client"
        } else { "daemon:type:server" };
        Ok(())
    }
}

impl<'a> Parameters<'a> {
    pub fn new(
        daemon_type: &'a DaemonType,
        verbosity: u8,
        debug: bool,
        system_config: Option<&String>,
        user_configs: Option<ValuesRef<String>>,
    ) -> Outcome<Self> {
        Parameters::create_log_dir(debug)?;
        Ok(Parameters {
            daemon_type,
            verbosity: match (debug, verbosity) {
                (true, _) => 4,
                (false, 0) => 2, // default to warn log level
                (_, v) => v,
            },
            clear_logs: if debug { true } else { false },
            debug,
            log_path: Parameters::get_log_path(debug, daemon_type),
            pid_path: Parameters::get_pid_path(debug, daemon_type),
            system_config: Parameters::resolve_system_config(daemon_type, system_config)?,
            user_configs: Parameters::resolve_user_configs(daemon_type, user_configs)?,
        })
    }

    fn create_log_dir(debug: bool) -> Outcome<()> {
        let path = if debug {
            Path::new("/tmp/sinkd")
        } else {
            Path::new("/var/log/sinkd")
        };

        if !path.exists() {
            if !debug && !have_permissions() {
                return bad!("Need elevated permissions to create /var/sinkd/");
            }
            match fs::create_dir_all(path) {
                Ok(_) => Ok(()),
                Err(e) => bad!("Unable to create '{}'  {}", path.display(), e),
            }
        } else {
            Ok(())
        }
    }

    fn get_log_path(debug: bool, daemon_type: &'a DaemonType) -> Arc<&Path> {
        match (debug, daemon_type) {
            (true, DaemonType::Client) => Arc::new(Path::new("/tmp/sinkd/client.log")),
            (true, DaemonType::Server) => Arc::new(Path::new("/tmp/sinkd/server.log")),
            (false, DaemonType::Client) => Arc::new(Path::new("/var/log/sinkd/client.log")),
            (false, DaemonType::Server) => Arc::new(Path::new("/var/log/sinkd/server.log")),
        }
    }

    fn get_pid_path(debug: bool, daemon_type: &'a DaemonType) -> Arc<&Path> {
        match (debug, daemon_type) {
            (true, DaemonType::Client) => Arc::new(Path::new("/tmp/sinkd/client.pid")),
            (true, DaemonType::Server) => Arc::new(Path::new("/tmp/sinkd/server.pid")),
            (false, DaemonType::Client) => Arc::new(Path::new("/var/log/sinkd/client.pid")),
            (false, DaemonType::Server) => Arc::new(Path::new("/var/log/sinkd/server.pid")),
        }
    }

    fn resolve_system_config(
        daemon_type: &'a DaemonType,
        system_config: Option<&String>,
    ) -> Outcome<Arc<PathBuf>> {
        if *daemon_type == DaemonType::Server {
            return Ok(Arc::new(PathBuf::from("not-used")));
        }
        // safe unwrap due to default args
        match resolve(system_config.unwrap()) {
            Ok(normalized) => {
                if normalized.is_dir() {
                    // TODO: have error codes
                    bad!(
                        "{} is a directory not a file, aborting",
                        normalized.display()
                    )
                } else {
                    Ok(Arc::new(normalized))
                }
            }
            Err(e) => bad!("system config path error: {}", e),
        }
    }

    // fn load_user_configs(user_configs: Option<ValuesRef<String>>) -> Outcome<Vec<PathBuf>> {
    //     match user_configs {
    //         Some(cfgs) => Ok(cfgs.map(|p| PathBuf::from(p)).collect()),
    //         None => Ok(vec![]), // server doesn't need user configs
    //     }
    // }

    pub fn resolve_user_configs(
        daemon_type: &'a DaemonType,
        user_configs: Option<ValuesRef<String>>,
    ) -> Outcome<Arc<Vec<PathBuf>>> {
        if *daemon_type == DaemonType::Server {
            return Ok(Arc::new(vec![PathBuf::from("not-used")]));
        }
        let mut resolved_configs = Vec::new();
        // safe unwrap due to default args
        if let Some(usr_cfgs) = user_configs {
            for cfg in usr_cfgs {
                let normalized = resolve(&cfg.to_string())?;
                if normalized.is_dir() {
                    return bad!(
                        "{} is a directory, not a file; aborting",
                        normalized.display()
                    );
                }
                resolved_configs.push(normalized);
            }
        }
        Ok(Arc::new(resolved_configs))
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
        info!("creating pid file: {}", params.pid_path.display());
        if let Err(why) = std::fs::File::create(*params.pid_path) {
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
    Ok(()) // already created
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
        create_pid_file(&params)?;
    }
    if pid == 0 {
        // if Parent process
        unsafe {
            // pid_file is typically set so unwrap here is safe
            let c_str = CString::new(params.pid_path.to_str().unwrap()).unwrap();
            // delete a name and possibly the file it refers to
            libc::unlink(c_str.into_raw());
        }
    } else {
        if let Err(e) = std::fs::write(*params.pid_path, pid.to_string()) {
            return bad!("couldn't write to '{}' {}", params.pid_path.display(), e);
        }
    }
    Ok(())
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

pub fn start_mosquitto() -> Outcome<()> {
    debug!(">> spawn mosquitto daemon");
    //? This command will not spawn new instances
    //? if mosquitto already active.
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
                        _ => set_pid(params, child.as_raw() as u32)?,
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

// pub fn end_process(params: &Parameters) -> Outcome<()> {
//     if !params.debug && !have_permissions() {
//         return bad!("Need to be root");
//     }
//     let pid = get_pid(params)?;
//     if let Err(e) = std::process::Command::new("kill")
//         .arg("-15")  //SIGTERM
//         .arg(format!("{}", pid))
//         .output()
//     {
//         return bad!("coudn't kill process {}", pid);
//     }
//     Ok(())
// }

pub fn end_process(params: &Parameters) -> Outcome<()> {
    if !params.debug && !have_permissions() {
        return bad!("Need to be root");
    }

    let pid = get_pid(params)?;
    let nix_pid = Pid::from_raw(pid as i32);

    match kill(nix_pid, Some(Signal::SIGTERM)) {
        Ok(_) => {
            // Process exists and can be signaled
            if let Err(e) = std::process::Command::new("kill")
                .arg("-15") // SIGTERM
                .arg(format!("{}", pid))
                .output()
            {
                return bad!("Couldn't kill process {} {}", pid, e);
            }
            Ok(())
        }
        Err(_) => {
            bad!(
                "Process with PID {} does not exist or cannot be signaled",
                pid
            )
        }
    }
}

pub fn rsync(payload: &ipc::Payload) {
    // Agnostic pathing allows sinkd not to care about user folder structure

    debug!("{}", payload);


    // TODO: 
    // FIXME 
    // NOTE: 
    // HACK: 
    // WARNING: 
    
    let pull = payload.status == ipc::Status::NotReady(ipc::Reason::Behind);

    let src: Vec<PathBuf> = if pull {
        payload.src_paths.iter().map(|p| PathBuf::from(
            format!("{}:{}", payload.hostname, p.display())
        )).collect()
    } else {
        payload.src_paths.clone()
    };

    // NOTE: this is assuming that dest will always be a single point 
    let dest: String = if pull {
        payload.dest_path.clone()
    } else {
        format!("{}:{}", payload.hostname, payload.dest_path)
    };

    // need to account for shared folders
    // and local sync? maybe useful for testing
    let mut cmd = std::process::Command::new("rsync"); // have to bind at .new()
    cmd.arg("-atR") // archive, timestamps, relative
        .arg("--delete") // delete on destination if not reflected in source
        //? RSYNC options to consider
        // .arg("--delete-excluded")
        // .arg("--max-size=SIZE") // (limit size of transfers)
        // .arg("--exclude=PATTERN") // loop through to all all paths
        .args(&src)
        .arg(&dest)
        ;
    

    match cmd.spawn() {
        Err(x) => {
            error!("{:?}", x);
        }
        Ok(_) => {
            debug!("called rsync! src_paths: {:?}  dest_path: {}", src, dest);
        }
    }
}

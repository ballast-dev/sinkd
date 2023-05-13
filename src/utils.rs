// Common Utilities
use libc::{c_char, c_uint};
use std::ffi::CString;
use std::path::PathBuf;
use std::process;
use std::sync::Mutex;

use crate::{fancy, ipc};

const TIMESTAMP_LENGTH: u8 = 25;

// TODO: move this into section of /etc/sinkd.conf
pub struct Parameters {
    pub verbosity: u8,
    pub clear_logs: bool,
    pub debug_mode: bool,
    log_path: PathBuf,
    pid_path: PathBuf,
}

impl Parameters {
    pub fn new() -> Self {
        Parameters {
            verbosity: 0,
            clear_logs: false,
            debug_mode: false,
            log_path: PathBuf::from("/run/sinkd.pid"),
            pid_path: PathBuf::from("/var/log/sinkd.log"),
        }
    }

    pub fn debug() -> Self {
        Parameters {
            verbosity: 3,
            clear_logs: true,
            debug_mode: true,
            log_path: PathBuf::from("/tmp/sinkd.pid"),
            pid_path: PathBuf::from("/tmp/sinkd.log"),
        }
    }

    // return a new copy of the path to the caller 
    pub fn get_log_path(&self) -> PathBuf {
        return self.log_path.clone();
    }

    pub fn get_pid_path(&self) -> PathBuf {
        return self.pid_path.clone();
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
    return String::from_utf8_lossy(&v).into_owned();
}

pub fn have_permissions() -> bool {
    unsafe {
        // get effective user id
        libc::geteuid() == 0
    }
}

pub fn create_pid_file(params: &Parameters) -> Result<(), String> {
    if !params.debug_mode && !have_permissions() {
        return Err(String::from("need to be root"));
    }
    let pid_file = params.get_pid_path();
    if !pid_file.exists() {
        // match std::fs::create_dir_all(&pid_file) {
        match std::fs::File::create(&pid_file) {
            Err(why) => {
                let err_str = format!("cannot create {:?}, {:?}", pid_file, why.kind());
                Err(err_str)
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

pub fn create_log_file(params: &Parameters) -> Result<(), String> {
    if !params.debug_mode && !have_permissions() {
        return Err(String::from("Need to be root to create log file"));
    }

    let log_file = params.get_log_path();
    if !log_file.exists() || params.clear_logs {
        if let Err(why) = std::fs::File::create(&log_file) {
            // truncates file if exists
            return Err(format!("cannot create {:?}, {:?}", log_file, why.kind()));
        }
    }
    Ok(()) // already created
}

pub fn get_pid(params: &Parameters) -> Result<u16, String> {
    // let user = env!("USER");
    // let sinkd_path = if cfg!(target_os = "macos") {
    //     path::Path::new("/Users").join(user).join(".sinkd")
    // } else {
    //     path::Path::new("/home").join(user).join(".sinkd")
    // };

    let pid_file = params.get_pid_path();

    if !pid_file.exists() {
        Err(String::from("pid file not found"))
    } else {
        match std::fs::read(&pid_file) {
            Err(err) => {
                let err_str = format!("Cannot read {}: {}", &pid_file.display(), err);
                Err(err_str)
            }
            Ok(contents) => {
                let pid_str = String::from_utf8_lossy(&contents);
                match pid_str.parse::<u16>() {
                    Err(e2) => {
                        let err_str = format!("Couldn't parse pid: {}", e2);
                        Err(err_str)
                    }
                    Ok(pid) => Ok(pid),
                }
            }
        }
    }
}

pub fn set_pid(params: &Parameters, pid: u16) -> Result<(), String> {
    let pid_file = params.get_pid_path();
    if !pid_file.exists() {
        return Err(String::from("pid file not found"));
    }

    if pid == 0 {
        unsafe {
            // pid_file is typically set so unwrap here is safe
            let c_str = CString::new(pid_file.to_str().unwrap()).unwrap();
            libc::unlink(c_str.into_raw());
        }
        Ok(())
    } else {
        match std::fs::write(pid_file, pid.to_ne_bytes()) {
            Err(err) => {
                let err_str = format!("couldn't clear pid in ~/.sinkd/pid\n{}", err);
                Err(err_str)
            }
            Ok(()) => Ok(()),
        }
    }
}

fn gen_keys() -> bool {
    let shell = process::Command::new("sh")
        .stdout(process::Stdio::null())
        .arg("-c")
        .arg("printf '\n\n' | ssh-keygen -t dsa") // will *not* overwrite
        .output()
        .unwrap();

    let shell_stderr = String::from_utf8_lossy(&shell.stderr);

    if shell_stderr.contains("not found") {
        println!("command not found: ssh-keygen, is ssh installed?");
        return false;
    }

    if shell_stderr.contains("already exist") {
        println!("keys already have been generated");
    } else {
        println!("generated key for current user");
    }
    true
}

fn copy_keys_to_remote(host: &str) -> bool {
    // todo: add an optional force flag '-f'

    let shell = process::Command::new("sh")
        .arg("-c")
        .arg(format!("ssh-copy-id -i ~/.ssh/id_ed25519.pub {}", host))
        .stdout(process::Stdio::null())
        .output()
        .unwrap();

    // let echo_stdout = String::from_utf8_lossy(&echo.stdout);
    let shell_stderr = String::from_utf8_lossy(&shell.stderr);

    if shell_stderr.contains("denied") {
        return false;
    }

    if shell_stderr.contains("already exist") {
        println!("ssh key already exist on server!");
        return true;
    } else {
        println!("ssh key loaded on remote system");
    }

    let shell = process::Command::new("sh")
        .arg("-c")
        .arg("eval $(ssh-agent) && ssh-add ~/.ssh/id_ed25519")
        .output()
        .unwrap();

    let shell_stderr = String::from_utf8_lossy(&shell.stderr);

    if shell_stderr.contains("not found") {
        println!("command not found: ssh-eval, is ssh installed?");
        false
    } else {
        println!("loaded private key with ssh-agent, passwordless login enabled!");
        true
    }
}

fn set_up_rsync_daemon(host: &str, pass: &str) {
    let connections = 5;
    let command_str = format!(
        r#"ssh -t {} << EOF
    local HISTSIZE=0  
    echo {} | sudo -Sk mkdir /srv/sinkd
    echo {} | sudo -Sk groupadd sinkd 
    echo {} | sudo -Sk chgrp sinkd /srv/sinkd
    echo {} | sudo -Sk tee /etc/rsyncd.conf << ENDCONF
uid = nobody
gid = nobody
use chroot = no
max connections = {}
syslog facility = local5
pid file = /run/rsyncd.pid

[sinkd]
    path = /srv/sinkd
    read only = false
    #gid = $GROUP

ENDCONF
    echo {} | sudo -Sk rsync --daemon
    EOF
    "#,
        host, pass, pass, pass, pass, connections, pass
    );

    let shell = process::Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .stdout(process::Stdio::null())
        .output()
        .unwrap();

    let shell_stderr = String::from_utf8_lossy(&shell.stderr);

    if shell_stderr.contains("not found") {
        println!("command not found: ssh-eval, is ssh installed?");
    } else if shell_stderr.contains("denied") {
        println!("access denied on remote, bad password?")
    } else {
        println!("loaded private key with ssh-agent, passwordless login enabled!");
        //~ Did it!
    }
}

pub fn setup_server(_verbosity: u8, host: &str) {
    let pass = rpassword::prompt_password("setting up daemon on server...\npassword: ").unwrap();
    set_up_rsync_daemon(host, &pass);
}

pub fn setup_keys(_verbosity: u8, host: &str) {
    if !gen_keys() {
        fancy::print_fancyln(
            "Unable to generate keys",
            fancy::Attrs::NORMAL,
            fancy::Colors::RED,
        );
        return;
    }

    if copy_keys_to_remote(host) {
        fancy::print_fancyln("finished setup", fancy::Attrs::NORMAL, fancy::Colors::GREEN)
    }

    // let mut du_output_child = Command::new("du")
    //     .arg("-ah")
    //     .arg(&directory)
    //     .stdout(Stdio::piped())
    //     .spawn()?;

    // if let Some(du_output) = du_output_child.stdout.take() {
    //     let mut sort_output_child = Command::new("sort")
    //         .arg("-hr")
    //         .stdin(du_output)
    //         .stdout(Stdio::piped())
    //         .spawn()?;

    //     du_output_child.wait()?;

    //     if let Some(sort_output) = sort_output_child.stdout.take() {
    //         let head_output_child = Command::new("head")
    //             .args(&["-n", "10"])
    //             .stdin(sort_output)
    //             .stdout(Stdio::piped())
    //             .spawn()?;

    //         let head_stdout = head_output_child.wait_with_output()?;

    //         sort_output_child.wait()?;

    //         println!(
    //             "Top 10 biggest files and directories in '{}':\n{}",
    //             directory.display(),
    //             String::from_utf8(head_stdout.stdout).unwrap()
    //         );
    //     }
    // }
}

/// Notify the sibling thread that a fatal condition has occured
/// Log error upon lock failure and exit immediately
/// TODO: make an optional parameter that will report on the error
pub fn fatal(mutex: &Mutex<bool>) {
    match mutex.lock() {
        Ok(mut cond) => *cond = true,
        Err(e) => {
            error!("FATAL couldn't unlock mutex aborting: {}", e);
            std::process::exit(1);
        }
    }
}

/// Based on Rust manual it is better practice to exit from main.
/// Firing up an Error all the way back to main is preferred to
/// clean up all heap and stack allocated memory.
pub fn abort(report: &str) -> Result<(), ()> {
    error!("{}", report);
    Err(())
}

/// Return status of shared mutex amoung threads
/// if unable to lock then return true which signifies program has exited
pub fn exited(mutex: &Mutex<bool>) -> bool {
    match mutex.lock() {
        Ok(cond) => *cond,
        Err(e) => {
            error!("FATAL couldn't unlock mutex aborting: {}", e);
            true
        }
    }
}

// use home_dir which should work on the *nixes
// if let Some(home) = env::home_dir() {
//     use crate::utils::{Attrs::*, Colors::*};
//     print_fancyln(format!("HOME{} ==>> print off environment", home.display()).as_str(), BOLD, GREEN);
//     for (key, value) in env::vars_os() {
//         println!("{:?}: {:?}", key, value);
//     }
// }

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

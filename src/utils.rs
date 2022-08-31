// Common Utilities
use libc::{c_char, c_uint};
use std::ffi::CString;
use std::fmt;
use std::path::PathBuf;
use std::process;
use std::sync::Mutex;

// TODO: need to wrap in os specific way
pub const PID_PATH: &str = "/run/sinkd.pid";
pub const LOG_PATH: &str = "/var/log/sinkd.log";

extern "C" {
    fn timestamp(ret_str: *mut c_char, size: c_uint, fmt_str: *const c_char);
}

pub fn get_timestamp(fmt_str: &str) -> String {
    let ret_str = CString::new(Vec::with_capacity(25)).unwrap();
    let ret_ptr: *mut c_char = ret_str.into_raw();

    let _fmt_str = CString::new(fmt_str.as_bytes()).unwrap();
    let stamp: CString;
    unsafe {
        timestamp(ret_ptr, 25, _fmt_str.as_ptr());
        stamp = CString::from_raw(ret_ptr);
    }
    let v = stamp.into_bytes();
    return String::from_utf8_lossy(&v).into_owned();
}

pub fn have_permissions() -> bool {
    unsafe {
        return libc::geteuid() == 0;
    }
}

pub fn create_pid_file() -> Result<(), String> {
    if !have_permissions() {
        return Err(String::from("Need to be root"));
    }
    let pid_file = PathBuf::from(PID_PATH);
    if !pid_file.exists() {
        // match std::fs::create_dir_all(&pid_file) {
        match std::fs::File::create(&pid_file) {
            Err(why) => {
                let err_str = format!("cannot create {:?}, {:?}", pid_file, why.kind());
                return Err(err_str);
            }
            Ok(_) => return Ok(()),
        }
    } else {
        return Ok(()); // already created
    }
    // fs::File::create(PID_FILE).expect("unable to create pid file, permissions?");
    // let metadata = pid_file.metadata().unwrap();
    // let mut permissions = metadata.permissions();
    // permissions.set_readonly(false);
    // fs::set_permissions(&pid_path, permissions).expect("cannot set permission");
}

pub fn create_log_file(clear: bool) -> Result<(), String> {
    if !have_permissions() {
        return Err(String::from("Need to be root"));
    }
    let log_file = PathBuf::from(LOG_PATH);
    if !log_file.exists() || clear {
        match std::fs::File::create(&log_file) {
            Err(why) => {
                let err_str = format!("cannot create {:?}, {:?}", log_file, why.kind());
                return Err(err_str);
            }
            Ok(_) => return Ok(()),
        }
    } else {
        return Ok(()); // already created
    }
}

pub fn get_pid() -> Result<u16, String> {
    // let user = env!("USER");
    // let sinkd_path = if cfg!(target_os = "macos") {
    //     path::Path::new("/Users").join(user).join(".sinkd")
    // } else {
    //     path::Path::new("/home").join(user).join(".sinkd")
    // };

    let pid_file = PathBuf::from(PID_PATH);

    if !pid_file.exists() {
        return Err(String::from("pid file not found"));
    } else {
        match std::fs::read(PID_PATH) {
            Err(err) => {
                let err_str = format!("Cannot read {}: {}", PID_PATH, err);
                return Err(err_str);
            }
            Ok(contents) => {
                let pid_str = String::from_utf8_lossy(&contents);
                match pid_str.parse::<u16>() {
                    Err(e2) => {
                        let err_str = format!("Couldn't parse pid: {}", e2);
                        return Err(err_str);
                    }
                    Ok(pid) => {
                        return Ok(pid);
                    }
                }
            }
        }
    }
}

pub fn set_pid(pid: u16) -> Result<(), String> {
    let pid_file = PathBuf::from(PID_PATH);
    if !pid_file.exists() {
        return Err(String::from("pid file not found"));
    }

    if pid == 0 {
        unsafe {
            let c_str = CString::new(PID_PATH).unwrap();
            libc::unlink(c_str.into_raw());
        }
        return Ok(());
    } else {
        match std::fs::write(pid_file, pid.to_ne_bytes()) {
            Err(err) => {
                let err_str = format!("couldn't clear pid in ~/.sinkd/pid\n{}", err);
                return Err(err_str);
            }
            Ok(()) => {
                return Ok(());
            }
        }
    }
}

//--------------------
// C O L O R S
//--------------------
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum Colors {
    // Foreground
    BLACK = 30,
    RED = 31,
    GREEN = 32,
    YELLOW = 33,
    BLUE = 34,
    PURPLE = 35,
    CYAN = 36,
    WHITE = 37,
    BRIGHT_BLUE = 94,
    BRIGHT_PURPLE = 95,
    // Background
    BgBLACK = 40,
    BgRED = 41,
    BgGREEN = 42,
    BgYELLOW = 43,
    BgBLUE = 44,
    BgPURPLE = 45,
    BgCYAN = 46,
    BgWHITE = 47,
}

impl fmt::Display for Colors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Colors::BLACK => write!(f, "30"),
            Colors::RED => write!(f, "31"),
            Colors::GREEN => write!(f, "32"),
            Colors::YELLOW => write!(f, "33"),
            Colors::BLUE => write!(f, "34"),
            Colors::PURPLE => write!(f, "35"),
            Colors::CYAN => write!(f, "36"),
            Colors::WHITE => write!(f, "37"),
            Colors::BRIGHT_BLUE => write!(f, "94"),
            Colors::BRIGHT_PURPLE => write!(f, "95"),
            Colors::BgBLACK => write!(f, "40"),
            Colors::BgRED => write!(f, "41"),
            Colors::BgGREEN => write!(f, "42"),
            Colors::BgYELLOW => write!(f, "43"),
            Colors::BgBLUE => write!(f, "44"),
            Colors::BgPURPLE => write!(f, "45"),
            Colors::BgCYAN => write!(f, "46"),
            Colors::BgWHITE => write!(f, "47"),
        }
    }
}

#[allow(dead_code)]
pub enum Attrs {
    // # Attributes
    NORMAL = 0,
    BOLD = 1,
    UNDERLINE = 4,
    INVERSE = 7, // foreground becomes background (vice-versa)
}

impl fmt::Display for Attrs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Attrs::NORMAL => write!(f, "0"),
            Attrs::BOLD => write!(f, "1"),
            Attrs::UNDERLINE => write!(f, "4"),
            Attrs::INVERSE => write!(f, "7"),
        }
    }
}

pub fn print_fancy(arg: &str, attr: Attrs, color: Colors) {
    print!("{}", format!("\u{1b}[{};{}m{}\u{1b}[0m", attr, color, arg));
}

pub fn print_fancyln(arg: &str, attr: Attrs, color: Colors) {
    println!("{}", format!("\u{1b}[{};{}m{}\u{1b}[0m", attr, color, arg));
}

pub fn format_fancy(arg: &str, attr: Attrs, color: Colors) -> String {
    format!("\u{1b}[{};{}m{}\u{1b}[0m", attr, color, arg)
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
    return true;
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
        return false;
    } else {
        println!("loaded private key with ssh-agent, passwordless login enabled!");
        return true;
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

pub fn setup_server(verbosity: u8, host: &str) {
    let pass = rpassword::prompt_password("setting up daemon on server...\npassword: ").unwrap();
    set_up_rsync_daemon(&host, &pass);
}

pub fn setup_keys(verbosity: u8, host: &str) {
    if !gen_keys() {
        print_fancyln("Unable to generate keys", Attrs::NORMAL, Colors::RED);
        return;
    }

    if copy_keys_to_remote(host) {
        print_fancyln("finished setup", Attrs::NORMAL, Colors::GREEN)
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
pub fn fatal(mutex: &Mutex<bool>) {
    match mutex.lock() {
        Ok(mut cond) => *cond = true,
        Err(e) => {
            error!("FATAL couldn't unlock mutex aborting: {}", e);
            std::process::exit(1);
        }
    }
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

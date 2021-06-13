use crate::{config, utils};
use crate::client::Client;
use daemonize::Daemonize;
use std::{fs, u8};
use libc;
use std::process;

pub const PID_FILE: &'static str = "/run/sinkd.pid";
pub const LOG_FILE: &'static str = "/var/log/sinkd.log";

fn reload_config() {
    info!("reload config?")
}

pub fn add(file: &str) -> bool {
    // adds entry to ~/.sinkd/sinkd.conf 
    // tells daemon to read config again
    // send a SIGHUP signal 
    // unsafe {
    //     let s: libc::sighandler_t = reload_config;
    //     libc::signal(libc::SIGHUP, s);
    // }
    println!("appending '{}' to watch files", file);
    return true; // able to watch directory
}

pub fn list() {
    //TODO need to list system shares
    let user = env!("USER");
    match config::Config::get_user_config(user) {
        Ok(usr_cfg) => {
            for anchor in &usr_cfg.anchors {
                println!("{}", anchor.path.display());
            }
        },
        Err(e) => {
            eprintln!("{}", e)
        }
    }
}

pub fn start() {
    process::Command::new("sh").arg("-c").arg("sudo systemctl start sinkd")
        .output()
        .unwrap();
}

/**
 * When sinkd is packaged should install /run/sinkd.pid file and make it writable the the sinkd group
 * Need to set up logging keep everything local to home directory ~/
 */
#[warn(unused_features)]
pub fn legacy_start() {
    // let sinkd_path = utils::get_sinkd_path();
    // let pid_path = sinkd_path.join("pid");

    // if !pid_path.exists() {
    //     let pid_file =
    //         fs::File::create(&pid_path).expect("unable to create pid file, permissions?");
    //     let metadata = pid_file.metadata().unwrap();
    //     let mut permissions = metadata.permissions();
    //     permissions.set_readonly(false);
    //     fs::set_permissions(&pid_path, permissions).expect("cannot set permission");
    // }

    // TODO: need packager to setup file with correct permisions
    let daemon = Daemonize::new()
        .pid_file(PID_FILE);
        // .chown_pid_file(true)  // is optional, see `Daemonize` documentation
        // .user("nobody")
        // .group("sinkd");

    match daemon.start() {
        Ok(_) => {
            info!("about to start daemon...");
            Client::new().init();
        }
        Err(e) => error!("sinkd did not start (already running?), {}", e),
    }
}

pub fn stop() {
    process::Command::new("sh").arg("-c").arg("sudo systemctl stop sinkd")
        // .stdout(process::Stdio::null())
        .output()
        .unwrap();
    // match std::fs::read("/run/sinkd.pid") {
    //     Err(err) => {
    //         eprintln!("Error stoping sinkd, {}", err);
    //         return;
    //     }
    //     Ok(contents) => {
    //         let pid_str = String::from_utf8_lossy(&contents);

    //         match pid_str.parse::<u32>() {
    //             Err(e2) => {
    //                 eprintln!("sinkd not running?");
    //                 eprintln!("{}", e2);
    //                 return;
    //             }
    //             Ok(pid) => {
    //                 println!("killing process {}", &pid);
    //                 std::process::Command::new("kill")
    //                     .arg("-15")
    //                     .arg(pid_str.as_ref())
    //                     .output()
    //                     .expect("ERROR couldn't kill daemon");
    //             }
    //         }
    //     }arg
    // }
    // match std::fs::write(PID_FILE, "") {
    //     Err(err) => eprintln!("couldn't clear pid in ~/.sinkd/pid\n{}", err),
    //     Ok(()) => println!("stopped sinkd daemon"),
    // }
}

pub fn restart() {
    process::Command::new("sh").arg("-c").arg("sudo systemctl restart sinkd")
        .output()
        .unwrap();
}

pub fn remove() {
    println!("remove files and folders")
}

pub fn log() {
    // info!("hello log");
    // warn!("warning");
    // error!("oops");
    print!(
        "{}",
        fs::read_to_string(LOG_FILE).expect("couldn't read log file, check permissions")
    );
}

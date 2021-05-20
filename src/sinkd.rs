use crate::utils;
use crate::client::Client;
use daemonize::Daemonize;
use std::{fs, u8};
use libc;

static PID_FILE: &str = "/run/sinkd.pid";

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
    println!("print out list of all watched folders")
}

/**
 * When sinkd is packaged should install /run/sinkd.pid file and make it writable the the sinkd group
 * Need to set up logging keep everything local to home directory ~/
 */
pub fn start() {
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
    match std::fs::read("/run/sinkd.pid") {
        Err(err) => {
            eprintln!("Error stoping sinkd, {}", err);
            return;
        }
        Ok(contents) => {
            let pid_str = String::from_utf8_lossy(&contents);

            match pid_str.parse::<u32>() {
                Err(e2) => {
                    eprintln!("sinkd not running?");
                    eprintln!("{}", e2);
                    return;
                }
                Ok(pid) => {
                    println!("killing process {}", &pid);
                    std::process::Command::new("kill")
                        .arg("-15")
                        .arg(pid_str.as_ref())
                        .output()
                        .expect("ERROR couldn't kill daemon");
                }
            }
        }
    }
    match std::fs::write(PID_FILE, "") {
        Err(err) => eprintln!("couldn't clear pid in ~/.sinkd/pid\n{}", err),
        Ok(()) => println!("stopped sinkd daemon"),
    }
}

pub fn restart() {
    stop();
    start();
}

pub fn remove() {
    println!("remove files and folders")
}

pub fn log() {
    // info!("hello log");
    // warn!("warning");
    // error!("oops");
    // shows the log

    let sinkd_path = utils::get_sinkd_path();
    let log_path = sinkd_path.join("log");
    print!(
        "{}",
        fs::read_to_string(log_path).expect("couldn't read log file, check permissions")
    );
}

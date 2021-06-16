use crate::{config, utils};
use crate::client::Client;
use daemonize::Daemonize;
use std::fs;
use crate::shiplog;

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

/**
 * When sinkd is packaged should install /run/sinkd.pid file and make it writable the the sinkd group
 * Need to set up logging keep everything local to home directory ~/
 */
// #[warn(unused_features)]
pub fn start() -> bool {

    match utils::create_log_file() {
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
        Ok(_) => { shiplog::ShipLog::init(); }
    }
    
    match utils::create_pid_file() {
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
        Ok(_) => {
            // TODO: need packager to setup file with correct permisions
            let daemon = Daemonize::new()
                .pid_file(utils::PID_PATH);
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
            return true;
        }
    }
}

pub fn stop() -> bool {
    if !utils::have_permissions() {
        eprintln!("Need to be root");
        return false;
    }
    match utils::get_pid() {
        Err(e) => { 
            eprintln!("{}", e);
            return false; 
        }
        Ok(pid) => {
            std::process::Command::new("kill")
                .arg("-15")
                .arg(format!("{}", pid))
                .output()
                .expect("ERROR couldn't kill daemon");
            println!("killed process {}", &pid);

            match utils::set_pid(0) {
                Err(e) => { 
                    eprintln!("{}", e); 
                    return false; 
                },
                Ok(_) => { return true;  }
            }
        }
    } 
}

pub fn restart() {
    if stop() {
        start();
    }
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
        fs::read_to_string(utils::LOG_PATH).expect("couldn't read log file, check permissions")
    );
}

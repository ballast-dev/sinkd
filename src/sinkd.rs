#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::shiplog;
use crate::{config, utils};
use daemonize::Daemonize;
use std::fs;

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

pub fn list<'a>(paths: Option<&Vec<&'a str>>) {
    //TODO need to list system shares
    match paths {
        Some(paths) => {
            for path in paths {
                println!("path: {}", path);
            }
            let user = env!("USER");
        },
        None => {
            println!("no paths were given!")
        }
    }
    // match config::ConfigParser::get_user_config(user) {
    //     Ok(usr_cfg) => {
    //         for anchor in &usr_cfg.anchors {
    //             println!("{}", anchor.path.display());
    //         }
    //     },
    //     Err(e) => {
    //         eprintln!("user config: {}", e)
    //     }
    // }
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
                }
                Ok(_) => {
                    return true;
                }
            }
        }
    }
}

pub fn restart() {
    // if stop() {
    //     start();
    // }
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

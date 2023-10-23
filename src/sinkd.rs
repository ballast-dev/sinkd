#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::outcome::Outcome;
use crate::shiplog;
use crate::utils::Parameters;
use crate::{config, utils};
use std::fs;

fn reload_config() {
    info!("reload config?")
}

pub fn add(share_paths: Vec<&String>, user_paths: Vec<&String>) -> Outcome<()> {
    // adds entry to ~/.sinkd/sinkd.conf
    // tells daemon to read config again
    // send a SIGHUP signal
    // unsafe {
    //     let s: libc::sighandler_t = reload_config;
    //     libc::signal(libc::SIGHUP, s);
    // }
    for p in &share_paths {
        println!("share.... {}", p);
    }
    for p in &user_paths {
        println!("user... {}", p);
    }
    Ok(())
}

pub fn list(paths: Option<&Vec<&str>>) -> Outcome<bool> {
    //TODO need to list system shares
    match paths {
        Some(paths) => {
            for path in paths {
                println!("path: {}", path);
            }
            let user = env!("USER");
            println!("under maintenance...");
            Ok(true)
        }
        None => bad!("no paths were given!"),
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

pub fn stop(params: &Parameters) -> bool {
    if !utils::have_permissions() {
        eprintln!("Need to be root");
        return false;
    }
    match utils::get_pid(params) {
        Err(e) => {
            eprintln!("{}", e);
            false
        }
        Ok(pid) => {
            std::process::Command::new("kill")
                .arg("-15")
                .arg(format!("{}", pid))
                .output()
                .expect("ERROR couldn't kill daemon");
            println!("killed process {}", &pid);

            match utils::set_pid(params, 0) {
                Err(e) => {
                    eprintln!("{}", e);
                    false
                }
                Ok(_) => true,
            }
        }
    }
}

pub fn restart() -> Outcome<bool> {
    // if stop() {
    //     start();
    // }
    Ok(true)
}

pub fn remove() -> Outcome<bool> {
    println!("remove files and folders");
    Ok(true)
}

pub fn log(params: &Parameters) -> Outcome<bool> {
    // info!("hello log");
    // warn!("warning");
    // error!("oops");
    print!(
        "{}",
        fs::read_to_string(*params.log_path).expect("couldn't read log file, check permissions")
    );
    Ok(true)
}

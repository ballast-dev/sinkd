use clap::parser::ValuesRef;
use std::fs;

use crate::{fancy_debug, outcome::Outcome, parameters::Parameters};
// adds entry to ~/.sinkd/sinkd.conf
// tells daemon to read config again
// send a SIGHUP signal
// unsafe {
//     let s: libc::sighandler_t = reload_config;
//     libc::signal(libc::SIGHUP, s);
// }

pub fn add(share_paths: Vec<&String>, user_paths: Vec<&String>) -> Outcome<()> {
    for p in &share_paths {
        println!("share_path: {p}");
    }
    for p in &user_paths {
        println!("user_path: {p}");
    }
    Ok(())
}

pub fn remove(share_paths: Vec<&String>, user_paths: Vec<&String>) -> Outcome<()> {
    for p in &share_paths {
        println!("share_path: {p}");
    }
    for p in &user_paths {
        println!("user_path: {p}");
    }
    Ok(())
}

pub fn adduser(users: Option<ValuesRef<String>>) -> Outcome<()> {
    match users {
        Some(users) => {
            for user in users {
                fancy_debug!("{}", user);
            }
            Ok(())
        }
        None => bad!("no user(s) were given!"),
    }
}

pub fn rmuser(users: Option<ValuesRef<String>>) -> Outcome<()> {
    match users {
        Some(users) => {
            for user in users {
                fancy_debug!("{}", user);
            }
            Ok(())
        }
        None => bad!("no user(s) were given!"),
    }
}

pub fn list(paths: Option<Vec<&String>>) -> Outcome<bool> {
    match paths {
        Some(paths) => {
            for path in paths {
                println!("path: {path}");
            }
            let user = std::env::var("USER").map_err(|e| e.to_string())?;
            fancy_debug!("user: {}", user);
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

pub fn log(params: &Parameters) -> Outcome<bool> {
    // info!("hello log");
    // warn!("warning");
    // error!("oops");
    print!(
        "{}",
        fs::read_to_string(&params.log_path).expect("couldn't read log file, check permissions")
    );
    Ok(true)
}

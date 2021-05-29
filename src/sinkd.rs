use crate::{config, utils};
use crate::client::Client;
use daemonize::Daemonize;
use std::{fs, path::PathBuf};
// use libc;

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
pub fn daemon() {
    Client::new().init();
    // let pid_path: PathBuf = PathBuf::from(PID_FILE);

    // if !pid_path.exists() {
    //     if let Err(e) = std::process::Command::new("touch").arg(PID_FILE).spawn() {
    //         error!("touch failed {}", e);
    //         panic!();
    //     }
    //     if let Err(e) = std::process::Command::new("chmod").arg("664").arg(PID_FILE).spawn() {
    //         error!("chmod failed {}", e);
    //         panic!();
    //     }

    //     if let Err(e) = std::process::Command::new("chown").arg("sinkd:sinkd").arg(PID_FILE).spawn() {
    //         error!("chown failed {}", e);
    //         panic!();
    //     }

    //     match fs::File::create(&pid_path) {
    //         Err(e) => {
    //             error!("trouble making pid file {}", e)
    //         },
    //         Ok(pid_file) => {
    //             // let metadata = pid_file.metadata().unwrap();
    //             // let mut permissions = metadata.permissions();
    //             // permissions.set_readonly(false);
    //             if let Err(e) = fs::set_permissions(&pid_path, fs::Permissions::from_mode(0o664)) {
    //                 error!("DID NOT SET PERMISSIONS!, {}", e);
    //                 panic!();
    //             }
    //         }
    //     }
    // }


    // TODO: need packager to setup file with correct permisions
    // let daemon = Daemonize::new()
    //     .pid_file(PID_FILE);

    // match daemon.start() {
    //     Ok(_) => {
    //         info!("about to start daemon...");
    //         Client::new().init();
    //     }
    //     Err(e) => error!("Error in creating daemon, {}", e),
    // }
}

// pub fn stop() {
//     match std::fs::read(PID_FILE) {
//         Err(err) => {
//             eprintln!("Error stoping sinkd, {}", err);
//             return;
//         }
//         Ok(contents) => {
//             let pid_str = String::from_utf8_lossy(&contents);

//             match pid_str.parse::<u32>() {
//                 Err(e2) => {
//                     eprintln!("sinkd not running?");
//                     eprintln!("{}", e2);
//                     return;
//                 }
//                 Ok(pid) => {
//                     println!("killing process {}", &pid);
//                     std::process::Command::new("kill")
//                         .arg("-15")
//                         .arg(pid_str.as_ref())
//                         .output()
//                         .expect("ERROR couldn't kill daemon");
//                 }
//             }
//         }
//     }
//     match std::fs::write(PID_FILE, "") {
//         Err(err) => eprintln!("couldn't clear pid in {}\n{}", PID_FILE, err),
//         Ok(()) => println!("stopped sinkd daemon"),
//     }
// }

// pub fn restart() {
//     stop();
//     start();
// }

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

use crate::utils;
use crate::client::Client;
use daemonize::Daemonize;
use std::{fs, u8};
use libc;


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
    // USER is an environment variable for *nix systems
    // NOTE: no intention to be used on windows
    let sinkd_path = utils::get_sinkd_path();
    let pid_path = sinkd_path.join("pid");

    if !pid_path.exists() {
        // then create file
        let pid_file =
            fs::File::create(&pid_path).expect("unable to create pid file, permissions?");
        let metadata = pid_file.metadata().unwrap();
        let mut permissions = metadata.permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&pid_path, permissions).expect("cannot set permission");
    }

    let daemon = Daemonize::new().pid_file(pid_path);
    ///// is the rest needed? having the pid within the users directory prevents ownership issues
    // .chown_pid_file(true)      // is optional, see `Daemonize` documentation
    // .working_directory("/etc/sinkd/") // for default behaviour.
    // .user("nobody")
    // .group("sinkd"); // Group name
    // .group(2)        // or group id.
    // .umask(0o777)    // Set umask, `0o027` by default.
    // .stdout(stdout)  // Redirect stdout to `/etc/sinkd/sinkd.out`.
    // .stderr(stderr)  // Redirect stderr to `/etc/sinkd/sinkd.err`.
    // .exit_action(|| println!("something?"))
    // .privileged_action(|| );

    match daemon.start() {
        Ok(_) => {
            info!("about to start daemon...");
            Client::new().init();
        }
        Err(e) => eprintln!("sinkd did not start (already running?), {}", e),
    }
}

pub fn stop() {
    let pid_path = utils::get_sinkd_path().join("pid");
    match std::fs::read(&pid_path) {
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
    match std::fs::write(&pid_path, "") {
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

use crate::ropework;
use crate::tradewind::Caravel;
use daemonize::Daemonize;
use std::fs;


pub fn add(file: &str) -> bool {
    // add to config file
    // 
    println!("appending '{}' to watch files", file);   
    return true; // able to watch directory
}

pub fn adduser(users: Vec<&str>) {
    println!("add {:?} to list of users who have permission to watch the directory", users)
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
    let sinkd_path = ropework::get_sinkd_path();
    let pid_path = sinkd_path.join("pid");

    if !pid_path.exists() { // then create file
        let pid_file = fs::File::create(&pid_path).expect("unable to create pid file, permissions?");
        let metadata = pid_file.metadata().unwrap();
        let mut permissions = metadata.permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&pid_path, permissions).expect("cannot set permission");
    }
    
    let daemonize = Daemonize::new()
        .pid_file(pid_path);
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
        // .privileged_action(|| Caravel::new().daemon());

    match daemonize.start() {
        Ok(_) => {
            Caravel::new().daemon();
            println!("sinkd started")
        },
        Err(e) => eprintln!("sinkd did not start (already running?), {}", e),
    }
}

pub fn stop() {
    let pid_path = ropework::get_sinkd_path().join("pid");
    match std::fs::read(&pid_path) {
        Err(err) => {
            eprintln!("Error stoping sinkd, {}", err);
            return;
        },
        Ok(contents) => {
            let pid_str = String::from_utf8_lossy(&contents);
            
            match pid_str.parse::<u32>() {
                Err(e2) => {
                    eprintln!("sinkd not running?");
                    eprintln!("{}", e2);
                    return;
                },
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
        Ok(()) =>   println!("stopped sinkd daemon")
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
    info!("hello log");
    warn!("warning");
    error!("oops");
}
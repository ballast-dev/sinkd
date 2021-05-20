/*
 * Command Line Interface
 */
use crate::daemon::barge::Barge;
use crate::daemon::harbor::Harbor;
use daemonize::Daemonize;
use clap::*;

pub enum DaemonType {
    Barge,
    Harbor,
}

pub fn build_cli() -> App<'static, 'static> {
    App::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("daemon")  // for debugging
            .short("d")
            .long("daemon")
            .hidden(true) // reentry point to spawn daemon for barge
        )
        .arg(Arg::with_name("harbor")
            .short("r")
            .long("harbor")
            .help("Spawns sinkd server")
        )
        .subcommand(App::new("add")
            .about("Adds PATH to watch list")
            .arg(Arg::with_name("PATH")
                .required(true)
                .help("sinkd starts watching path")
            )
            .help("usage: sinkd add FILE [OPTIONS]\n\
                lets sinkd become 'aware' of file or folder location provided")
        )
        .subcommand(App::new("adduser")
            .about("Add USER to watch")
            .arg(Arg::with_name("[USER, ...]")
                .required(true)
                .help("sinkd adduser USER")
            )
            .help("usage: sinkd adduser USER")
        )
        .subcommand(App::new("ls")
            .alias("list")
            .about("List currently watched files from given PATH")
            .arg(Arg::with_name("PATH")
                .required(false)
                .help("list watched files and directories")
            )
            .help("usage: sinkd ls [PATH]")
        )
        .subcommand(App::new("rm")
            .alias("remove")
            .about("Removes PATH from list of watched directories")
            .arg(Arg::with_name("PATH")
                .required(true)
            )
            .help("usage: sinkd rm PATH")
        )
        .subcommand(App::new("rmuser")
            .about("Removes USER from watch")
            .arg(Arg::with_name("USER")
                .required(true)
            )
            .help("usage: sinkd rmuser USER")
        )
        .subcommand(App::new("start")
            .about("Starts the daemon")
        )
        .subcommand(App::new("stop")
            .about("Stops daemon")
        )
        .subcommand(App::new("restart")
            .about("Restarts sinkd, reloading configuration")
        )
 
}


pub fn add(daemon_type: DaemonType, file: String) -> bool {

    match daemon_type {
        DaemonType::Barge => {
            println!("appending '{}' to watch files", file);   
            let mut barge = Barge::new(); 
            barge.anchor(file, 1, Vec::new()); 
            return true; // able to watch directory
        },
        DaemonType::Harbor => {
            // stuff for server
            println!("anchor in for harbor");
            return true;
        }
    }
}

pub fn adduser(users: Vec<&str>) {
    println!("add {:?} to list of users who have permission to watch the directory",
             users)
}

pub fn list() {
    // list all current files loaded
    println!("print out list of all watched folders")
}

pub fn start() {
    // let stdout = File::create("/etc/sinkd/sinkd.out").unwrap();
    // let stderr = File::create("/etc/sinkd/sinkd.err").unwrap();
    // File::create("/run/sinkd.pid").expect("probably permissons"); // need to create file as a package

    let daemonize = Daemonize::new()
        .pid_file("/run/sinkd.pid"); // Every method except `new` and `start`
        // .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        // .working_directory("/etc/sinkd/") // for default behaviour.
        // .user("nobody")
        // .group("sinkd"); // Group name
        // .group(2)        // or group id.
        // .umask(0o777)    // Set umask, `0o027` by default.
        // .stdout(stdout)  // Redirect stdout to `/etc/sinkd/sinkd.out`.
        // .stderr(stderr)  // Redirect stderr to `/etc/sinkd/sinkd.err`.
        // .exit_action(|| println!("something?"))
        // .privileged_action(|| Barge::new().daemon());

    match daemonize.start() {
        Ok(_) => {
            Barge::new().daemon();
        },
        Err(e) => eprintln!("Error, {}", e),
    }
}

pub fn stop() {
    println!("stopping daemon");

    let sinkd_pid: String = String::from_utf8_lossy(&std::fs::read("/run/sinkd.pid").unwrap()).parse().unwrap();
    // need to keep pid of barge process in separate file
    std::process::Command::new("kill")
                           .arg("-15")
                           .arg(sinkd_pid)
                           .output()
                           .expect("ERROR couldn't kill daemon");
}

pub fn restart() {
    stop();
    start();
}

pub fn remove() {
    println!("remove files and folders")
}


// the same daemon should run on both machines ( in the same place )
pub fn daemon() {
    // Spawns in shell, stays in shell
    // match daemon_type {
    //     DaemonType::Barge => println!("starting barge"),
    //     DaemonType::Harbor => println!("starting harbor"),
    // }
    Barge::new().daemon()
}

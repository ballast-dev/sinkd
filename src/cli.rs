/*
 * Command Line Interface
 */
use crate::daemon::barge::Barge;
use crate::daemon::harbor::Harbor;
use daemonize::Daemonize;
use clap::*;
use std::fs;

pub enum DaemonType {
    Barge,
    Harbor,
}

pub fn build_cli() -> App<'static, 'static> {
    App::new("sinkd")
        .about("deployable cloud")
        .version(env!("CARGO_PKG_VERSION"))
        // TODO revisit, maybe a better approach
        .arg(Arg::with_name("daemon")  // for debugging
            .short("d")
            .long("daemon")
            .hidden(true) // reentry point to spawn daemon for barge
        )
        .subcommand(App::new("harbor")
        // harbor by default will print off configuration
            .about("Control the harbor daemon (on server)")
            .arg(Arg::with_name("dock")
                .short("d")
                .long("dock")
                .help("Generates configuration and starts harbor daemon")
            )
            .arg(Arg::with_name("start")
                .long("start")
                .help("starts sinkd harbor (server)")
            )       
            .arg(Arg::with_name("stop")
                .long("stop")
                .help("stops sinkd harbor (server)")
            )       
            .help("All harbor commands should be invoked on the 'server'\ncontrol the server side daemon (hoster of files)")
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

/**
 * When sinkd is packaged should install /run/sinkd.pid file and make it writable the the sinkd group
 * Need to set up logging keep everything local to home directory ~/
 */
pub fn start() {
    let user = env!("USER");
    let home_dir = if cfg!(target_os = "macos") {
        std::path::Path::new("/Users").join(user)
    } else {
        std::path::Path::new("/home").join(user)
    };    
    let sinkd_path = home_dir.join(".sinkd");
    println!("{:?}", sinkd_path);
    match fs::create_dir(&sinkd_path) {
        Err(why) => println!("cannot create dir => {:?}", why.kind()),
        Ok(_) => {},
    }
    let pid_path = sinkd_path.join("pid");
    // fs::create_dir(path).unwrap_or(println!("uh oh....")); // already created return empty unit

    // need to use correct path ~ is not interpretted
    let pid_file = fs::File::open(&pid_path).unwrap_or(
        fs::File::create(&pid_path).expect("Unable to create file")
    );
    let metadata = pid_file.metadata().unwrap();
    let mut permissions = metadata.permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&pid_path, permissions).expect("cannot set permission");
    
    println!("daemonize?");

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
        // .privileged_action(|| Barge::new().daemon());

    match daemonize.start() {
        Ok(_) => {
            println!("started daemon!");
            Barge::new().daemon();
        },
        Err(e) => eprintln!("Error Daemonize, {}", e),
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

use crate::ropework;
use crate::tradewind::Windjammer;
use daemonize::Daemonize;
use std::fs;
use std::process;

fn gen_keys() -> bool {
    let shell = process::Command::new("sh")
        .stdout(process::Stdio::null())
        .arg("-c")
        .arg("printf '\n\n' | ssh-keygen -t dsa")  // will *not* overwrite 
        .output()
        .unwrap();

    let shell_stderr = String::from_utf8_lossy(&shell.stderr);

    if shell_stderr.contains("not found") {
        println!("command not found: ssh-keygen, is ssh installed?");
        return false;
    }

    if shell_stderr.contains("already exist") {
        println!("keys already have been generated");
    } else {
        println!("generated key for current user");
    }
    return true;
}

fn copy_keys_to_remote(host: &str) -> bool {
    // todo: add an optional force flag '-f'

    let shell = process::Command::new("sh")
        .arg("-c")
        .arg(format!("ssh-copy-id -i ~/.ssh/id_ed25519.pub {}", host))
        .stdout(process::Stdio::null())
        .output()
        .unwrap();

    // let echo_stdout = String::from_utf8_lossy(&echo.stdout);
    let shell_stderr = String::from_utf8_lossy(&shell.stderr);

    if shell_stderr.contains("denied") {
        return false;
    }

    if shell_stderr.contains("already exist") {
        println!("ssh key already exist on server!");
        return true;
    } else {
        println!("ssh key loaded on remote system");
    }


    let shell = process::Command::new("sh")
        .arg("-c")
        .arg("eval $(ssh-agent) && ssh-add ~/.ssh/id_ed25519")
        .output()
        .unwrap();
        
    let shell_stderr = String::from_utf8_lossy(&shell.stderr);
    
    if shell_stderr.contains("not found") {
        println!("command not found: ssh-eval, is ssh installed?");
        return false;
    } else {
        println!("loaded private key with ssh-agent, passwordless login enabled!");
        return true;
    }
}

fn set_up_rsync_daemon(host: &str, pass: &str) {
    let connections = 5;
    let command_str = format!(r#"ssh -t {} << EOF
    local HISTSIZE=0  
    echo {} | sudo -Sk mkdir /srv/sinkd
    echo {} | sudo -Sk groupadd sinkd 
    echo {} | sudo -Sk chgrp sinkd /srv/sinkd
    echo {} | sudo -Sk tee /etc/rsyncd.conf << ENDCONF
uid = nobody
gid = nobody
use chroot = no
max connections = {}
syslog facility = local5
pid file = /run/rsyncd.pid

[sinkd]
    path = /srv/sinkd
    read only = false
    #gid = $GROUP

ENDCONF
    echo {} | sudo -Sk rsync --daemon
    EOF
    "#, host, pass, pass, pass, pass, connections, pass);

    let shell = process::Command::new("sh")
    .arg("-c")
    .arg(command_str)
    .stdout(process::Stdio::null())
    .output()
    .unwrap();

    let shell_stderr = String::from_utf8_lossy(&shell.stderr);
    
    if shell_stderr.contains("not found") {
        println!("command not found: ssh-eval, is ssh installed?");
    } else if shell_stderr.contains("denied") {
        println!("access denied on remote, bad password?")
    } else {
        println!("loaded private key with ssh-agent, passwordless login enabled!");
        //~ Did it!
    }

}

pub fn setup_server(verbosity: u8, host: &str) {
    let pass = rpassword::prompt_password_stdout("setting up daemon on server...\npassword: ").unwrap();
    set_up_rsync_daemon(&host, &pass);
}

pub fn setup_keys(verbosity: u8, host: &str) {
    if gen_keys() {
        if copy_keys_to_remote(host) {
           ropework::print_fancyln("finished setup", ropework::Attrs::NORMAL, ropework::Colors::GREEN)
        }
    }
    
    // let mut du_output_child = Command::new("du")
    //     .arg("-ah")
    //     .arg(&directory)
    //     .stdout(Stdio::piped())
    //     .spawn()?;

    // if let Some(du_output) = du_output_child.stdout.take() {
    //     let mut sort_output_child = Command::new("sort")
    //         .arg("-hr")
    //         .stdin(du_output)
    //         .stdout(Stdio::piped())
    //         .spawn()?;

    //     du_output_child.wait()?;

    //     if let Some(sort_output) = sort_output_child.stdout.take() {
    //         let head_output_child = Command::new("head")
    //             .args(&["-n", "10"])
    //             .stdin(sort_output)
    //             .stdout(Stdio::piped())
    //             .spawn()?;

    //         let head_stdout = head_output_child.wait_with_output()?;

    //         sort_output_child.wait()?;

    //         println!(
    //             "Top 10 biggest files and directories in '{}':\n{}",
    //             directory.display(),
    //             String::from_utf8(head_stdout.stdout).unwrap()
    //         );
    //     }
    // }
}

pub fn add(file: &str) -> bool {
    // add to config file
    //
    println!("appending '{}' to watch files", file);
    return true; // able to watch directory
}

pub fn adduser(users: Vec<&str>) {
    println!(
        "add {:?} to list of users who have permission to watch the directory",
        users
    )
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

    if !pid_path.exists() {
        // then create file
        let pid_file =
            fs::File::create(&pid_path).expect("unable to create pid file, permissions?");
        let metadata = pid_file.metadata().unwrap();
        let mut permissions = metadata.permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&pid_path, permissions).expect("cannot set permission");
    }

    let daemonize = Daemonize::new().pid_file(pid_path);
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

    match daemonize.start() {
        Ok(_) => {
            info!("about to start daemon...");
            Windjammer::new().trawl();
        }
        Err(e) => eprintln!("sinkd did not start (already running?), {}", e),
    }
}

pub fn stop() {
    let pid_path = ropework::get_sinkd_path().join("pid");
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

    let sinkd_path = ropework::get_sinkd_path();
    let log_path = sinkd_path.join("log");
    print!(
        "{}",
        fs::read_to_string(log_path).expect("couldn't read log file, check permissions")
    );
}

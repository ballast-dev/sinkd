use crate::ropework;
use crate::tradewind::Caravel;
use daemonize::Daemonize;
use std::fs;
use std::process;

fn gen_keys() -> bool {
    let shell = process::Command::new("sh")
        .stdout(process::Stdio::null())
        .arg("-c")
        .arg("printf '\n\n' | ssh-keygen -t dsa")  // will *not* overwrite 
        .output();
    match shell {
        Err(x) => {
            println!("unable to generate ssh keys, is ssh installed?\n{:?}", x.to_string());
            return false;
        }
        Ok(_) => {
            println!("generated keys for current user");
            return true;
        }
    }
}

fn copy_keys_to_remote(pass: &str, host: &str) -> bool {
    let command_str = format!("ssh-copy-id -i ~/.ssh/id_ed25519.pub {}", host);
    println!("what are we putting out? {}", command_str);

    let mut echo = process::Command::new("sh")
        .arg("-c")
        .arg(format!("echo '{}'", pass))
        .stdout(process::Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(echo_output) = echo.stdout.take() {
        println!("{:?}", echo_output);
        let copyid = process::Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .stdin(echo_output)
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .spawn()
        .unwrap();

        let copy_output = copyid.wait_with_output();

        if let Ok(_) = echo.wait() {

            match copy_output {
                Err(x) => {
                    println!("unable to copy ssh keys to remote, check firewall and ensure sshd is running!\n{:?}", x.to_string());
                    return false;
                }
                Ok(output) => {
                    println!("copied ssh-keys onto remote machine");
                    println!("STDOUT: {:?}", String::from_utf8_lossy(&output.stdout));
                    println!("STDERR: {:?}", String::from_utf8_lossy(&output.stderr));
                    return true;
                }
            }

        } else {
            false
        }   

    } else {
        return false; // todo: need to handle better
    }
}



pub fn init(host: &str) {
    if gen_keys() {
        let pass = rpassword::prompt_password_stdout("setting ssh-keys on remote...\npassword: ").unwrap();
        copy_keys_to_remote(&pass, host);

        
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
    // .privileged_action(|| Caravel::new().daemon());

    match daemonize.start() {
        Ok(_) => {
            info!("about to start daemon...");
            Caravel::new().daemon();
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

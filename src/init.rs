use crate::utils;
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
           utils::print_fancyln("finished setup", utils::Attrs::NORMAL, utils::Colors::GREEN)
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

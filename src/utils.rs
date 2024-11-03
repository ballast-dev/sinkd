use std::path::{Path, PathBuf};

use crate::{ipc, outcome::Outcome};

pub fn have_permissions() -> bool {
    unsafe {
        // get effective user id
        libc::geteuid() == 0
    }
}

/// Both macOS and Linux have the uname command
pub fn get_hostname() -> String {
    match std::process::Command::new("uname").arg("-n").output() {
        Err(e) => {
            error!("uname didn't work? {}", e);
            String::from("uname-error")
        }
        Ok(output) => {
            let mut v = output.stdout.to_ascii_lowercase();
            v.truncate(v.len() - 1); // strip newline
            debug!("{}", std::str::from_utf8(&v).unwrap());
            String::from_utf8(v).unwrap_or_else(|_| {
                error!("invalid string from uname -a");
                String::from("invalid-hostname")
            })
        }
    }
}

/// Both macOS and Linux have the whoami command
pub fn get_username() -> String {
    match std::process::Command::new("whoami").output() {
        Err(e) => {
            error!("whoami didn't work? {}", e);
            String::from("whoami error")
        }
        Ok(output) => {
            let mut v = output.stdout.to_ascii_lowercase();
            v.truncate(v.len() - 1); // strip newline
            debug!("{}", std::str::from_utf8(&v).unwrap());
            String::from_utf8(v).unwrap_or_else(|_| {
                error!("invalid string from whoami");
                String::from("invalid-username")
            })
        }
    }
}

// this will resolve all known paths, converts relative to absolute
pub fn resolve(path: &str) -> Outcome<PathBuf> {
    // NOTE: `~` is a shell expansion not handled by system calls
    if path.starts_with("~/") {
        let mut p = match std::env::var("HOME") {
            Ok(home_dir) => PathBuf::from(home_dir),
            Err(e) => {
                return bad!("HOME env var not defined: {}", e);
            }
        };
        p.push(&path.strip_prefix("~/").unwrap());
        match p.canonicalize() {
            Ok(resolved) => Ok(resolved),
            Err(e) => bad!("{} '{}'", e, p.display()),
        }
    } else {
        match PathBuf::from(path).canonicalize() {
            Ok(resolved) => Ok(resolved),
            Err(e) => bad!("{} '{}'", e, path),
        }
    }
}

// pub fn end_process(params: &Parameters) -> Outcome<()> {
//     if !params.debug && !have_permissions() {
//         return bad!("Need to be root");
//     }
//     let pid = get_pid(params)?;
//     if let Err(e) = std::process::Command::new("kill")
//         .arg("-15")  //SIGTERM
//         .arg(format!("{}", pid))
//         .output()
//     {
//         return bad!("coudn't kill process {}", pid);
//     }
//     Ok(())
// }

use nix::unistd::{fork, setsid, ForkResult};
use std::os::fd::IntoRawFd;

pub fn daemon(func: fn(&Parameters) -> Outcome<()>, params: &Parameters) -> Outcome<()> {
        match unsafe { fork() } {
            Ok(ForkResult::Parent { .. }) => Ok(()),
            Ok(ForkResult::Child) => {
                if setsid().is_err() {
                    return bad!("ipc::daemon >> Failed to create a new session");
                }
                match unsafe { fork() } {
                    Ok(ForkResult::Parent { .. }) => Ok(()),
                    Ok(ForkResult::Child) => {
                        if let Err(e) = std::env::set_current_dir("/") {
                            return bad!(format!("Failed to change working directory to root: {e}"));
                        }
                        unsafe { libc::umask(0o022) };
                        redirect_stdio_to_null();
                        shiplog::init(params)?;
                        func(params)
                    }
                    Err(_) => bad!("Second fork failed"),
                }
            }
            Err(_) => bad!("First fork failed"),
        }
}

#[cfg(unix)]
fn redirect_stdio_to_null() {
    let devnull = File::open("/dev/null").expect("Failed to open /dev/null");
    unsafe {
        let devnull_fd = devnull.into_raw_fd();
        libc::dup2(devnull_fd, libc::STDIN_FILENO);
        libc::dup2(devnull_fd, libc::STDOUT_FILENO);
        libc::dup2(devnull_fd, libc::STDERR_FILENO);
    }
}
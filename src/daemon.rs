use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{fork, ForkResult};

pub fn detach() {
    // TODO: need packager to setup file with correct permisions

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            let start_time = Instant::now();
            let timeout = Duration::from_secs(2);

            while start_time.elapsed() < timeout {
                match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                    Ok(status) => match status {
                        WaitStatus::Exited(_, _) => return bad!("client encountered error"),
                        _ => (),
                    },
                    Err(e) => eprintln!("Failed to wait on child?: {}", e),
                }
                std::thread::sleep(Duration::from_secs(1));
            }
            println!("spawned, logging to '{}'", params.log_path.display());
            Ok(())
        }
        Ok(ForkResult::Child) => {
            info!("about to start daemon...");
            //TODO: pass in a function pointer
            init(params)
        }
        Err(_) => {
            bad!("Failed to fork process")
        }
    }
}

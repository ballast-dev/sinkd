/// Windows daemonizing is re-entrant meaning the entry point 
/// of the executable is started afresh with no context from 
/// the parent. Passing a hidden flag into the original executable
/// seems to be the idiomatic approach on windows. Thus the main
/// of this program checks for the "hidden flag" at the start.
fn main() {
    // Check if this process is the "daemonized" child by looking for the flag.
    if std::env::args().any(|arg| arg == "--windows-daemon") {
        child();
    } else {
        parent();
    }
}

const CHILD_FILE: &str = "windows_daemon_child.txt";

fn child() {
    // In a well behaved daemon, the child process doesn't output to stdio
    if let Err(e) = daemon::redirect_stdio_to_null() {
        eprintln!("Could not redirect stdio: {e}");
    }

    match std::fs::write(CHILD_FILE, "Hello from Windows daemonized child!") {
        Ok(_) => println!("stdio has been redirected, you should not see this!"),
        Err(e) => println!("Or this! {e:?}")
    }
}

fn parent() {
    println!("Parent: Attempting to daemonize...");

    match daemon::daemon(None) {
        Ok(pid) => {
            println!("Parent: Successfully spawned child with PID: {pid:?}");
            println!("Check '{CHILD_FILE}' to see if the child wrote to it.");
        }
        Err(e) => {
            eprintln!("Parent: Failed to daemonize. Error: {e}");
        }
    }
}

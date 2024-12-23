#[cfg(target_os = "windows")]
pub fn is_process_running(process_name: &str) -> bool {
    use std::process::Command;

    let output = Command::new("tasklist")
        .arg("/fo")
        .arg("csv")
        .arg("/nh")
        .output()
        .expect("Failed to execute tasklist");

    let output_str = String::from_utf8_lossy(&output.stdout);
    output_str.contains(process_name)
}

#[cfg(target_os = "macos")]
pub fn is_process_running(process_name: &str) -> bool {
    use std::process::Command;

    let output = Command::new("pgrep")
        .arg("-q")
        .arg(process_name)
        .output()
        .expect("Failed to execute pgrep");

    output.status.success()
}

#[cfg(target_os = "linux")]
fn is_process_running(process_name: &str) -> bool {
    use std::process::Command;

    let output = Command::new("pgrep")
        .arg("-q")
        .arg(process_name)
        .output()
        .expect("Failed to execute pgrep");

    output.status.success()
}

fn main() {
    let process_name = "your_process_name";

    if is_process_running(process_name) {
        println!("Process '{}' is running.", process_name);
    } else {
        println!("Process '{}' is not running.", process_name);
    }
}

// On many Linux distributions, pgrep is provided as part of the procps-ng package.
// You can usually install it using the package manager for your specific distribution.
// For example, on Ubuntu or Debian, you can install pgrep with the following command:
//
// sudo apt-get install procps
//
// brew install proctools (sounds like macOS has this be default)

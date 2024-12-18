mod env;
mod event;

use log::{error, info};
use std::fs;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use env::Environment;

fn main() -> Result<(), String> {
    env_logger::init();
    build_sinkd().expect("Failed to build sinkd");
    let env = Environment::setup();
    let server = spawn_server();
    let client = spawn_client(&env);
    run_scenario(&env).expect("Failed to run the situation");
    sleep(Duration::from_secs(10)); // for the server to pick up the change
    stop_sinkd().expect("Failed to stop sinkd");
    wait_for_exit(server).expect("Failed to wait for server process");
    wait_for_exit(client).expect("Failed to wait for client process");
    info!("Testing completed successfully.");
    Ok(())
}

fn build_sinkd() -> Result<(), String> {
    info!("Building sinkd...");
    let status = Command::new("cargo")
        .arg("build")
        .current_dir("../sinkd") // Specify the relative path to sinkd
        .status()
        .expect("Failed to execute cargo build");
    if !status.success() {
        return Err(format!("cargo build failed with status: {}", status));
    }
    info!("sinkd built successfully.");
    Ok(())
}

/// Runs the testing scenario by manipulating files.
fn run_scenario(env: &Environment) -> Result<(), String> {
    info!("Running test situation...");
    let single_file_path = env.repo_root.join("test").join("single_file");
    let mut single_file = fs::File::create(&single_file_path).expect(&format!(
        "Failed to create file {:?}",
        &single_file_path.display()
    ));

    let folder1 = env.client_path.join("folder1");
    env::remove_subfiles(&folder1)?;
    event::create_files(&folder1, 3, 0.5).expect("Failed to create files in folder1");

    let folder2 = env.client_path.join("folder2");
    env::remove_subfiles(&folder2)?;
    event::create_files(&folder2, 10, 1.0).expect("Failed to create files in folder2");

    fs::File::write(&mut single_file, b"thingy").expect("cannot write to file");

    info!("==>> Finished client situation <<==");
    Ok(())
}

/// Spawns the sinkd server process.
fn spawn_server() -> Child {
    info!("Spawning sinkd server...");
    let child = Command::new("../sinkd/target/debug/sinkd") // Path to sinkd binary
        .arg("-d")
        .arg("server")
        .arg("start")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn sinkd server");
    // Optional: Add a delay to ensure the server starts properly
    sleep(Duration::from_secs(2));
    child
}

/// Spawns the sinkd client process with specified configurations.
fn spawn_client(env: &Environment) -> Child {
    info!("Spawning sinkd client...");
    let child = Command::new("../sinkd/target/debug/sinkd")
        .arg("-d")
        .arg("client")
        .arg("--sys-cfg")
        .arg(&env.server_config)
        .arg("--usr-cfg")
        .arg(&env.client_config)
        .arg("start")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn sinkd client");
    // Optional: Add a delay to ensure the client starts properly
    sleep(Duration::from_secs(2));
    child
}

/// Stops both the sinkd client and server processes.
fn stop_sinkd() -> Result<(), String> {
    info!("Stopping sinkd client and server...");
    let client_status = Command::new("../sinkd/target/debug/sinkd")
        .arg("-d")
        .arg("client")
        .arg("stop")
        .status()
        .expect("Failed to execute stop command for sinkd client");
    if !client_status.success() {
        return Err(format!(
            "Failed to stop sinkd client with status: {}",
            client_status
        ));
    }

    let server_status = Command::new("../sinkd/target/debug/sinkd")
        .arg("-d")
        .arg("server")
        .arg("stop")
        .status()
        .expect("Failed to execute stop command for sinkd server");
    if !server_status.success() {
        return Err(format!(
            "Failed to stop sinkd server with status: {}",
            server_status
        ));
    }
    info!("sinkd client and server stopped.");
    Ok(())
}

/// Waits for a child process to exit and logs its status.
fn wait_for_exit(mut child: Child) -> Result<(), String> {
    let status = child.wait().expect("Failed to wait for child process");
    if status.success() {
        info!("Process exited successfully.");
    } else {
        error!("Process exited with status: {}", status);
    }
    Ok(())
}

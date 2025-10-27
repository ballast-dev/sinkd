mod env;
mod event;

use log::{error, info};
use std::env::args;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use env::Environment;

fn main() {
    env_logger::init();

    let args: Vec<String> = args().collect();
    let instance_type = if args.len() > 1 {
        args[1].as_str()
    } else {
        "default"
    };

    info!("Starting scenario for instance type: {instance_type}");

    match instance_type {
        "alpha" => run_alpha_scenario(),
        "bravo" => run_bravo_scenario(),
        "charlie" => run_charlie_scenario(),
        _ => run_default_scenario(),
    }
}

fn run_alpha_scenario() {
    info!("Running Alpha (Server) scenario");
    let mut server = spawn_server_alpha();

    // Keep server running and wait for termination signal
    loop {
        sleep(Duration::from_secs(5));
        // Check if server is still running
        if let Ok(Some(_)) = server.try_wait() {
            error!("Server process exited unexpectedly");
            break;
        }
        // In a real scenario, you'd check for termination conditions
    }

    // Wait for server to finish
    let _ = server.wait();
}

fn run_bravo_scenario() {
    info!("Running Bravo (Client) scenario - file creator");
    let mut client = spawn_client_bravo();

    // Wait for client to start
    sleep(Duration::from_secs(5));

    // Create files in shared directory
    create_bravo_files();

    // Keep client running
    loop {
        sleep(Duration::from_secs(10));
        // Check if client is still running
        if let Ok(Some(_)) = client.try_wait() {
            error!("Client process exited unexpectedly");
            break;
        }
        // Periodically create more files
        modify_bravo_files();
    }

    // Wait for client to finish
    let _ = client.wait();
}

fn run_charlie_scenario() {
    info!("Running Charlie (Client) scenario - file modifier");
    let mut client = spawn_client_charlie();

    // Wait for client to start and for bravo to create files
    sleep(Duration::from_secs(10));

    // Modify files created by bravo
    loop {
        sleep(Duration::from_secs(15));
        // Check if client is still running
        if let Ok(Some(_)) = client.try_wait() {
            error!("Client process exited unexpectedly");
            break;
        }
        modify_charlie_files();
    }

    // Wait for client to finish
    let _ = client.wait();
}

fn run_default_scenario() {
    info!("Running default scenario (original behavior)");
    build_sinkd().expect("Failed to build sinkd");
    let env = Environment::setup();
    let server = spawn_server();
    let client = spawn_client(&env);
    run_scenario(&env);
    sleep(Duration::from_secs(10)); // for the server to pick up the change
    stop_sinkd().expect("Failed to stop sinkd");
    wait_for_exit(server);
    wait_for_exit(client);
    info!("Testing completed successfully.");
}

fn build_sinkd() -> Result<(), String> {
    info!("Building sinkd...");
    let status = Command::new("cargo")
        .arg("build")
        .current_dir("../sinkd") // Specify the relative path to sinkd
        .status()
        .expect("Failed to execute cargo build");
    if !status.success() {
        return Err(format!("cargo build failed with status: {status}"));
    }
    info!("sinkd built successfully.");
    Ok(())
}

/// Runs the testing scenario by manipulating files.
fn run_scenario(env: &Environment) {
    info!("Running test situation...");
    let single_file_path = env.repo_root.join("test").join("single_file");
    let mut single_file = fs::File::create(&single_file_path)
        .unwrap_or_else(|_| panic!("Failed to create file {:?}", &single_file_path.display()));

    let folder1 = env.client_path.join("folder1");
    env::remove_subfiles(&folder1);
    event::create_files(&folder1, 3, 0.5);

    let folder2 = env.client_path.join("folder2");
    env::remove_subfiles(&folder2);
    event::create_files(&folder2, 10, 1.0);

    fs::File::write(&mut single_file, b"thingy").expect("cannot write to file");

    info!("==>> Finished client situation <<==");
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
            "Failed to stop sinkd client with status: {client_status}"
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
            "Failed to stop sinkd server with status: {server_status}"
        ));
    }
    info!("sinkd client and server stopped.");
    Ok(())
}

/// Waits for a child process to exit and logs its status.
fn wait_for_exit(mut child: Child) {
    let status = child.wait().expect("Failed to wait for child process");
    if status.success() {
        info!("Process exited successfully.");
    } else {
        error!("Process exited with status: {status}");
    }
}

// New functions for Docker scenario instances

/// Spawns the sinkd server for alpha instance
fn spawn_server_alpha() -> Child {
    info!("Spawning sinkd server (alpha)...");
    let child = Command::new("/usr/local/bin/sinkd")
        .arg("server")
        .arg("start")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn sinkd server");
    sleep(Duration::from_secs(3));
    child
}

/// Spawns the sinkd client for bravo instance
fn spawn_client_bravo() -> Child {
    info!("Spawning sinkd client (bravo)...");
    let child = Command::new("/usr/local/bin/sinkd")
        .arg("client")
        .arg("start")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn sinkd client");
    sleep(Duration::from_secs(3));
    child
}

/// Spawns the sinkd client for charlie instance
fn spawn_client_charlie() -> Child {
    info!("Spawning sinkd client (charlie)...");
    let child = Command::new("/usr/local/bin/sinkd")
        .arg("client")
        .arg("start")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn sinkd client");
    sleep(Duration::from_secs(3));
    child
}

/// Creates initial files for bravo scenario
fn create_bravo_files() {
    info!("Bravo creating initial files...");

    // Create bravo's directory
    fs::create_dir_all("/shared/bravo").expect("Failed to create bravo directory");
    fs::create_dir_all("/shared/common").expect("Failed to create common directory");

    // Create some initial files
    for i in 0..5 {
        let file_path = format!("/shared/bravo/bravo_file_{i}.txt");
        fs::write(&file_path, format!("Initial content from bravo - file {i}"))
            .expect("Failed to create bravo file");
        info!("Created: {file_path}");

        sleep(Duration::from_secs(1));
    }

    // Create a shared file
    let shared_file = "/shared/common/shared_document.txt";
    fs::write(
        shared_file,
        "This is a shared document created by bravo\nLine 2\n",
    )
    .expect("Failed to create shared file");
    info!("Created shared file: {shared_file}");
}

/// Periodically modifies files for bravo scenario
fn modify_bravo_files() {
    info!("Bravo modifying files...");

    // Add a new file periodically
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let new_file = format!("/shared/bravo/bravo_periodic_{timestamp}.txt");
    fs::write(
        &new_file,
        format!("Periodic file created by bravo at {timestamp}"),
    )
    .expect("Failed to create periodic file");
    info!("Bravo created periodic file: {new_file}");

    // Modify the shared document
    if let Ok(mut content) = fs::read_to_string("/shared/common/shared_document.txt") {
        writeln!(content, "Bravo update at {timestamp}").expect("Failed to format string");
        fs::write("/shared/common/shared_document.txt", content)
            .expect("Failed to update shared document");
        info!("Bravo updated shared document");
    }
}

/// Modifies files created by bravo for charlie scenario
fn modify_charlie_files() {
    info!("Charlie modifying files...");

    // Create charlie's directory
    fs::create_dir_all("/shared/charlie").expect("Failed to create charlie directory");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create charlie's own files
    let charlie_file = format!("/shared/charlie/charlie_response_{timestamp}.txt");
    fs::write(
        &charlie_file,
        format!("Charlie's response file created at {timestamp}"),
    )
    .expect("Failed to create charlie file");
    info!("Charlie created: {charlie_file}");

    // Modify bravo's files if they exist
    if let Ok(entries) = fs::read_dir("/shared/bravo") {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str()
                && file_name.starts_with("bravo_file_")
                && std::path::Path::new(file_name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
            {
                let file_path = entry.path();
                if let Ok(mut content) = fs::read_to_string(&file_path) {
                    write!(content, "\n--- Modified by Charlie at {timestamp} ---\n")
                        .expect("Failed to format string");
                    if fs::write(&file_path, content).is_ok() {
                        info!("Charlie modified: {}", file_path.display());
                        break; // Only modify one file per cycle
                    }
                }
            }
        }
    }

    // Modify the shared document
    if let Ok(mut content) = fs::read_to_string("/shared/common/shared_document.txt") {
        writeln!(content, "Charlie's contribution at {timestamp}")
            .expect("Failed to format string");
        fs::write("/shared/common/shared_document.txt", content)
            .expect("Failed to update shared document");
        info!("Charlie updated shared document");
    }
}

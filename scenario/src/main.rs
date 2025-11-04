mod report;

use log::{error, info};
use std::env::args;
use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

const TEST_SCENARIOS_PATH: &str = "/mounted_path/test_scenarios";

fn main() {
    env_logger::init();

    let args: Vec<String> = args().collect();
    let instance_type = if args.len() > 1 {
        args[1].as_str()
    } else {
        error!("Usage: scenario <alpha|bravo|charlie|delta>");
        std::process::exit(1);
    };

    info!("Starting scenario for instance type: {instance_type}");

    match instance_type {
        "alpha" => run_alpha_scenario(),
        "bravo" => run_bravo_scenario(),
        "charlie" => run_charlie_scenario(),
        "delta" => run_delta_scenario(),
        _ => {
            error!("Unknown instance type: {instance_type}. Must be one of: alpha, bravo, charlie, delta");
            std::process::exit(1);
        }
    }
}

fn run_alpha_scenario() {
    info!("Running Alpha (Server) scenario");
    info!("Alpha server should already be running from docker-compose command");
    
    // Alpha just keeps running - server is managed by docker-compose
    loop {
        sleep(Duration::from_secs(30));
        info!("Alpha server scenario still running...");
    }
}

fn run_bravo_scenario() {
    info!("Running Bravo (Client) scenario - file creator/modifier");
    info!("Bravo client should already be running from docker-compose command");
    
    // Wait for client to start and sync to initialize
    sleep(Duration::from_secs(10));

    // Create initial files
    create_bravo_files();

    // Keep client running and periodically modify files
    loop {
        sleep(Duration::from_secs(10));
        // Periodically create/modify more files
        modify_bravo_files();
    }
}

fn run_charlie_scenario() {
    info!("Running Charlie (Client) scenario - file modifier");
    info!("Charlie client should already be running from docker-compose command");
    
    // Wait for client to start and for bravo to create files
    sleep(Duration::from_secs(15));

    // Modify files created by bravo
    loop {
        sleep(Duration::from_secs(15));
        modify_charlie_files();
    }
}

fn run_delta_scenario() {
    info!("Running Delta (Client) scenario - reporter");
    info!("Delta client should already be running from docker-compose command");
    
    // Wait for client to start and sync to initialize
    sleep(Duration::from_secs(20));

    // Periodically generate reports
    let mut report_counter = 0;
    loop {
        sleep(Duration::from_secs(30));
        
        report_counter += 1;
        info!("Delta generating report #{report_counter}");
        if let Err(e) = report::generate_toml_report(TEST_SCENARIOS_PATH, report_counter) {
            error!("Failed to generate report: {e}");
        }
    }
}


/// Creates initial files for bravo scenario
fn create_bravo_files() {
    info!("Bravo creating initial files in {TEST_SCENARIOS_PATH}...");

    let base_path = Path::new(TEST_SCENARIOS_PATH);
    let bravo_path = base_path.join("bravo");
    fs::create_dir_all(&bravo_path).expect("Failed to create bravo directory");

    // Create some initial files with bravo_ prefix
    for i in 0..5 {
        let file_path = bravo_path.join(format!("bravo_file_{i}.txt"));
        fs::write(&file_path, format!("Initial content from bravo - file {i}\n"))
            .expect("Failed to create bravo file");
        info!("Created: {}", file_path.display());
        sleep(Duration::from_secs(1));
    }

    // Create a shared file that both bravo and charlie can modify
    let shared_file = base_path.join("shared_document.txt");
    fs::write(
        &shared_file,
        "Initial shared document\nLine 2: Original content\n",
    )
    .expect("Failed to create shared file");
    info!("Created shared file: {}", shared_file.display());
}

/// Periodically modifies files for bravo scenario
fn modify_bravo_files() {
    info!("Bravo modifying files...");

    let base_path = Path::new(TEST_SCENARIOS_PATH);
    let bravo_path = base_path.join("bravo");
    fs::create_dir_all(&bravo_path).expect("Failed to ensure bravo directory exists");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Add a new file periodically
    let new_file = bravo_path.join(format!("bravo_periodic_{timestamp}.txt"));
    fs::write(
        &new_file,
        format!("Periodic file created by bravo at {timestamp}\n"),
    )
    .expect("Failed to create periodic file");
    info!("Bravo created periodic file: {}", new_file.display());

    // Modify the shared document
    let shared_file = base_path.join("shared_document.txt");
    if shared_file.exists() {
        if let Ok(mut content) = fs::read_to_string(&shared_file) {
            content.push_str(&format!("\n--- BRAVO MODIFICATION at {} ---\n", timestamp));
            fs::write(&shared_file, content)
                .expect("Failed to update shared document");
            info!("Bravo updated shared document");
        }
    } else {
        // Create if it doesn't exist
        fs::write(
            &shared_file,
            format!("Shared document (created by bravo at {})\n", timestamp),
        )
        .expect("Failed to create shared file");
    }
}

/// Modifies files created by bravo for charlie scenario
fn modify_charlie_files() {
    info!("Charlie modifying files...");

    let base_path = Path::new(TEST_SCENARIOS_PATH);
    let charlie_path = base_path.join("charlie");
    fs::create_dir_all(&charlie_path).expect("Failed to create charlie directory");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create charlie's own files
    let charlie_file = charlie_path.join(format!("charlie_response_{timestamp}.txt"));
    fs::write(
        &charlie_file,
        format!("Charlie's response file created at {timestamp}\n"),
    )
    .expect("Failed to create charlie file");
    info!("Charlie created: {}", charlie_file.display());

    // Modify bravo's files if they exist (synced via sinkd)
    let bravo_path = base_path.join("bravo");
    if bravo_path.exists() {
        if let Ok(entries) = fs::read_dir(&bravo_path) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str()
                    && file_name.starts_with("bravo_file_")
                    && entry.path().extension().is_some_and(|ext| ext == "txt")
                {
                    let file_path = entry.path();
                    if let Ok(mut content) = fs::read_to_string(&file_path) {
                        content.push_str(&format!("\n--- Modified by Charlie at {} ---\n", timestamp));
                        if fs::write(&file_path, content).is_ok() {
                            info!("Charlie modified: {}", file_path.display());
                            break; // Only modify one file per cycle
                        }
                    }
                }
            }
        }
    }

    // Modify the shared document (synced via sinkd)
    let shared_file = base_path.join("shared_document.txt");
    if shared_file.exists() {
        if let Ok(mut content) = fs::read_to_string(&shared_file) {
            content.push_str(&format!("\n--- CHARLIE MODIFICATION at {} ---\n", timestamp));
            fs::write(&shared_file, content)
                .expect("Failed to update shared document");
            info!("Charlie updated shared document");
        }
    }
}

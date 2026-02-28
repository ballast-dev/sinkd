mod report;
mod runner;
mod spec;

use clap::Parser;
use log::{error, info};
use runner::ScenarioRunner;
use spec::parse_spec;
use std::{env, fs, path::Path, path::PathBuf, thread::sleep, time::Duration};

const TEST_SCENARIOS_PATH: &str = "/mounted_path/test_scenarios";

#[derive(Parser, Debug)]
#[command(name = "scenario")]
#[command(about = "Declarative scenario test harness runner")]
struct Args {
    /// Path to scenario spec TOML file
    #[arg(short, long, default_value = "scenario/specs/local_smoke.toml")]
    spec: PathBuf,
    /// Root directory where scenario actions are executed
    #[arg(short, long, default_value = "test_scenarios/harness")]
    root: PathBuf,
}

fn main() {
    env_logger::init();

    // Backward-compatible role mode used by docker-compose.
    if let Some(role) = env::args().nth(1)
        && matches!(role.as_str(), "alpha" | "bravo" | "charlie" | "delta")
    {
        run_legacy_role(&role);
        return;
    }

    let args = Args::parse();

    let contents = match fs::read_to_string(&args.spec) {
        Ok(contents) => contents,
        Err(e) => {
            error!("unable to read spec '{}': {}", args.spec.display(), e);
            std::process::exit(1);
        }
    };
    let spec = match parse_spec(&contents) {
        Ok(spec) => spec,
        Err(e) => {
            error!("{e}");
            std::process::exit(1);
        }
    };

    info!(
        "running scenario '{}' from '{}' at root '{}'",
        spec.name,
        args.spec.display(),
        args.root.display()
    );
    let runner = ScenarioRunner::new(args.root);
    if let Err(e) = runner.run(&spec) {
        error!("{e}");
        std::process::exit(1);
    }
}

fn run_legacy_role(role: &str) {
    info!("running legacy role mode: {role}");
    match role {
        "alpha" => loop {
            sleep(Duration::from_secs(30));
            info!("alpha role heartbeat");
        },
        "bravo" => {
            sleep(Duration::from_secs(10));
            create_bravo_files();
            loop {
                sleep(Duration::from_secs(10));
                modify_bravo_files();
            }
        }
        "charlie" => {
            sleep(Duration::from_secs(15));
            loop {
                sleep(Duration::from_secs(15));
                modify_charlie_files();
            }
        }
        "delta" => {
            sleep(Duration::from_secs(20));
            let mut report_counter = 0;
            loop {
                sleep(Duration::from_secs(30));
                report_counter += 1;
                if let Err(e) = crate::report::generate_toml_report(TEST_SCENARIOS_PATH, report_counter)
                {
                    error!("Failed to generate report: {e}");
                }
            }
        }
        _ => {}
    }
}

fn create_bravo_files() {
    let base_path = Path::new(TEST_SCENARIOS_PATH);
    let bravo_path = base_path.join("bravo");
    fs::create_dir_all(&bravo_path).expect("Failed to create bravo directory");

    for i in 0..5 {
        let file_path = bravo_path.join(format!("bravo_file_{i}.txt"));
        fs::write(
            &file_path,
            format!("Initial content from bravo - file {i}\n"),
        )
        .expect("Failed to create bravo file");
        sleep(Duration::from_secs(1));
    }

    let shared_file = base_path.join("shared_document.txt");
    fs::write(
        &shared_file,
        "Initial shared document\nLine 2: Original content\n",
    )
    .expect("Failed to create shared file");
}

fn modify_bravo_files() {
    let base_path = Path::new(TEST_SCENARIOS_PATH);
    let bravo_path = base_path.join("bravo");
    fs::create_dir_all(&bravo_path).expect("Failed to ensure bravo directory exists");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let new_file = bravo_path.join(format!("bravo_periodic_{timestamp}.txt"));
    fs::write(
        &new_file,
        format!("Periodic file created by bravo at {timestamp}\n"),
    )
    .expect("Failed to create periodic file");

    let shared_file = base_path.join("shared_document.txt");
    if shared_file.exists() {
        if let Ok(mut content) = fs::read_to_string(&shared_file) {
            use std::fmt::Write;
            let _ = writeln!(content, "\n--- BRAVO MODIFICATION at {timestamp} ---");
            let _ = fs::write(&shared_file, content);
        }
    } else {
        let _ = fs::write(
            &shared_file,
            format!("Shared document (created by bravo at {timestamp})\n"),
        );
    }
}

fn modify_charlie_files() {
    let base_path = Path::new(TEST_SCENARIOS_PATH);
    let charlie_path = base_path.join("charlie");
    fs::create_dir_all(&charlie_path).expect("Failed to create charlie directory");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let charlie_file = charlie_path.join(format!("charlie_response_{timestamp}.txt"));
    let _ = fs::write(
        &charlie_file,
        format!("Charlie's response file created at {timestamp}\n"),
    );

    let bravo_path = base_path.join("bravo");
    if bravo_path.exists()
        && let Ok(entries) = fs::read_dir(&bravo_path)
    {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str()
                && file_name.starts_with("bravo_file_")
                && entry.path().extension().is_some_and(|ext| ext == "txt")
            {
                let file_path = entry.path();
                if let Ok(mut content) = fs::read_to_string(&file_path) {
                    use std::fmt::Write;
                    let _ = writeln!(content, "\n--- Modified by Charlie at {timestamp} ---");
                    let _ = fs::write(&file_path, content);
                    break;
                }
            }
        }
    }

    let shared_file = base_path.join("shared_document.txt");
    if shared_file.exists()
        && let Ok(mut content) = fs::read_to_string(&shared_file)
    {
        use std::fmt::Write;
        let _ = writeln!(content, "\n--- CHARLIE MODIFICATION at {timestamp} ---");
        let _ = fs::write(&shared_file, content);
    }
}

mod runner;
mod spec;

use clap::Parser;
use log::error;
use runner::ScenarioRunner;
use spec::parse_spec;
use std::{fs, path::PathBuf};

#[derive(Parser, Debug)]
#[command(name = "scenario")]
#[command(about = "Declarative scenario test harness runner")]
struct Args {
    /// Path to scenario spec TOML file
    #[arg(short, long, default_value = "scenario/specs/compose.toml")]
    spec: PathBuf,
    /// Root directory where scenario actions are executed
    #[arg(short, long, default_value = "scenario/compose")]
    root: PathBuf,
}

fn main() {
    env_logger::init();

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

    log::info!(
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

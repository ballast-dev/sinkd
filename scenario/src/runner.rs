use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread::sleep,
    time::{Duration, Instant},
};

use log::info;
use serde::Serialize;

use crate::spec::{ScenarioSpec, Step};

fn expand_root_template(root: &Path, s: &str) -> String {
    let resolved = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    s.replace("{{root}}", &resolved.display().to_string())
}

#[inline]
fn millis_as_u64(ms: u128) -> u64 {
    u64::try_from(ms).unwrap_or(u64::MAX)
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create parent '{}': {e}", parent.display()))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub struct ScenarioRunner {
    root: PathBuf,
}

impl ScenarioRunner {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        ScenarioRunner {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn run(&self, spec: &ScenarioSpec) -> Result<(), String> {
        let timeout = if spec.timeout_ms == 0 {
            Duration::from_secs(30)
        } else {
            Duration::from_millis(spec.timeout_ms)
        };
        let started = Instant::now();

        info!("running scenario '{}'", spec.name);
        let mut report = ScenarioReport::new(spec.name.clone(), &self.root);
        let mut overall_result: Result<(), String> = Ok(());
        for (idx, step) in spec.steps.iter().enumerate() {
            if started.elapsed() > timeout {
                overall_result = Err(format!(
                    "scenario '{}' exceeded timeout of {:?}",
                    spec.name, timeout
                ));
                break;
            }
            let step_started = Instant::now();
            match self.execute_step(step) {
                Ok(()) => report.steps.push(ExecutedStep {
                    index: idx,
                    kind: step.kind(),
                    elapsed_ms: millis_as_u64(step_started.elapsed().as_millis()),
                    success: true,
                    error: None,
                }),
                Err(e) => {
                    report.steps.push(ExecutedStep {
                        index: idx,
                        kind: step.kind(),
                        elapsed_ms: millis_as_u64(step_started.elapsed().as_millis()),
                        success: false,
                        error: Some(e.clone()),
                    });
                    overall_result = Err(e);
                    break;
                }
            }
        }

        report.total_elapsed_ms = millis_as_u64(started.elapsed().as_millis());
        report.success = overall_result.is_ok();
        report.error = overall_result.clone().err();
        let _ = self.write_report(&report);

        if overall_result.is_ok() {
            info!("scenario '{}' completed successfully", spec.name);
        }
        overall_result
    }

    #[allow(clippy::too_many_lines)]
    fn execute_step(&self, step: &Step) -> Result<(), String> {
        match step {
            Step::CreateDir { path } => {
                let full = self.root.join(path);
                fs::create_dir_all(&full)
                    .map_err(|e| format!("failed to create dir '{}': {e}", full.display()))
            }
            Step::WriteFile { path, content } => {
                let full = self.root.join(path);
                ensure_parent_dir(&full)?;
                fs::write(&full, content)
                    .map_err(|e| format!("failed to write file '{}': {e}", full.display()))
            }
            Step::AppendFile { path, content } => {
                use std::io::Write;

                let full = self.root.join(path);
                ensure_parent_dir(&full)?;
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&full)
                    .map_err(|e| format!("failed to open file '{}': {e}", full.display()))?;
                file.write_all(content.as_bytes())
                    .map_err(|e| format!("failed to append file '{}': {e}", full.display()))
            }
            Step::SleepMs { duration_ms } => {
                sleep(Duration::from_millis(*duration_ms));
                Ok(())
            }
            Step::RunCommand {
                command,
                allow_failure,
            } => {
                let command = expand_root_template(&self.root, command);
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .status()
                    .map_err(|e| format!("failed to execute command '{command}': {e}"))?;
                if status.success() || *allow_failure {
                    Ok(())
                } else {
                    Err(format!(
                        "command '{command}' failed with status {:?}",
                        status.code()
                    ))
                }
            }
            Step::AssertExists { path } => {
                let full = self.root.join(path);
                if full.exists() {
                    Ok(())
                } else {
                    Err(format!("expected path to exist: '{}'", full.display()))
                }
            }
            Step::AssertContains { path, contains } => {
                let full = self.root.join(path);
                let content = fs::read_to_string(&full)
                    .map_err(|e| format!("failed reading '{}': {e}", full.display()))?;
                if content.contains(contains) {
                    Ok(())
                } else {
                    Err(format!(
                        "expected '{}' to contain '{}'",
                        full.display(),
                        contains
                    ))
                }
            }
            Step::AssertEventuallyExists {
                path,
                within_ms,
                poll_interval_ms,
            } => Self::assert_eventually(
                || self.root.join(path).exists(),
                *within_ms,
                *poll_interval_ms,
                &format!(
                    "expected path to eventually exist: '{}'",
                    self.root.join(path).display()
                ),
            ),
            Step::AssertEventuallyContains {
                path,
                contains,
                within_ms,
                poll_interval_ms,
            } => {
                let full = self.root.join(path);
                Self::assert_eventually(
                    || {
                        fs::read_to_string(&full)
                            .map(|text| text.contains(contains))
                            .unwrap_or(false)
                    },
                    *within_ms,
                    *poll_interval_ms,
                    &format!(
                        "expected '{}' to eventually contain '{}'",
                        full.display(),
                        contains
                    ),
                )
            }
        }
    }

    fn assert_eventually<F>(
        mut predicate: F,
        within_ms: u64,
        poll_interval_ms: u64,
        message: &str,
    ) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        let started = Instant::now();
        let timeout = Duration::from_millis(within_ms);
        let poll = Duration::from_millis(poll_interval_ms.max(1));
        while started.elapsed() < timeout {
            if predicate() {
                return Ok(());
            }
            sleep(poll);
        }
        if predicate() {
            Ok(())
        } else {
            Err(message.to_string())
        }
    }

    fn write_report(&self, report: &ScenarioReport) -> Result<(), String> {
        let artifacts = self.root.join("_artifacts");
        fs::create_dir_all(&artifacts).map_err(|e| {
            format!(
                "failed to create artifacts dir '{}': {e}",
                artifacts.display()
            )
        })?;
        let output = artifacts.join("latest.toml");
        let serialized = toml::to_string_pretty(report)
            .map_err(|e| format!("failed to serialize report: {e}"))?;
        fs::write(&output, serialized)
            .map_err(|e| format!("failed to write report '{}': {e}", output.display()))?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct ScenarioReport {
    name: String,
    root: String,
    success: bool,
    total_elapsed_ms: u64,
    error: Option<String>,
    steps: Vec<ExecutedStep>,
}

impl ScenarioReport {
    fn new(name: String, root: impl AsRef<Path>) -> Self {
        ScenarioReport {
            name,
            root: root.as_ref().display().to_string(),
            success: false,
            total_elapsed_ms: 0,
            error: None,
            steps: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
struct ExecutedStep {
    index: usize,
    kind: &'static str,
    elapsed_ms: u64,
    success: bool,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{ScenarioRunner, expand_root_template};
    use crate::spec::parse_spec;

    #[test]
    fn expand_root_template_replaces_placeholder() {
        let tmp = std::env::temp_dir().join(format!(
            "sinkd_scenario_tpl_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).expect("mkdir");
        let resolved = tmp.canonicalize().expect("canonicalize");
        let out = expand_root_template(&tmp, "pre {{root}} post");
        assert!(
            out.contains(resolved.to_str().expect("utf8")),
            "expected '{}' in '{}'",
            resolved.display(),
            out
        );
        let _ = std::fs::remove_dir_all(tmp);
    }

    /// Runs `scenario/specs/local_smoke.toml` so step definitions stay in one place.
    #[test]
    fn local_smoke_spec_runs() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let spec_path = manifest.join("specs/local_smoke.toml");
        let raw = fs::read_to_string(&spec_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", spec_path.display()));
        let spec = parse_spec(&raw).expect("parse local_smoke.toml");

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sinkd_local_smoke_{unique}"));

        let runner = ScenarioRunner::new(&root);
        runner.run(&spec).expect("local_smoke scenario should pass");
        let _ = std::fs::remove_dir_all(&root);
    }
}

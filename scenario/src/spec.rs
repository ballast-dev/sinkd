use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ScenarioSpec {
    pub name: String,
    #[serde(default)]
    pub timeout_ms: u64,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    CreateDir {
        path: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    AppendFile {
        path: String,
        content: String,
    },
    SleepMs {
        duration_ms: u64,
    },
    RunCommand {
        command: String,
        #[serde(default)]
        allow_failure: bool,
    },
    AssertExists {
        path: String,
    },
    AssertContains {
        path: String,
        contains: String,
    },
    AssertEventuallyExists {
        path: String,
        within_ms: u64,
        #[serde(default = "default_poll_interval_ms")]
        poll_interval_ms: u64,
    },
    AssertEventuallyContains {
        path: String,
        contains: String,
        within_ms: u64,
        #[serde(default = "default_poll_interval_ms")]
        poll_interval_ms: u64,
    },
}

impl Step {
    pub fn kind(&self) -> &'static str {
        match self {
            Step::CreateDir { .. } => "create_dir",
            Step::WriteFile { .. } => "write_file",
            Step::AppendFile { .. } => "append_file",
            Step::SleepMs { .. } => "sleep_ms",
            Step::RunCommand { .. } => "run_command",
            Step::AssertExists { .. } => "assert_exists",
            Step::AssertContains { .. } => "assert_contains",
            Step::AssertEventuallyExists { .. } => "assert_eventually_exists",
            Step::AssertEventuallyContains { .. } => "assert_eventually_contains",
        }
    }
}

fn default_poll_interval_ms() -> u64 {
    200
}

pub fn parse_spec(contents: &str) -> Result<ScenarioSpec, String> {
    toml::from_str(contents).map_err(|e| format!("unable to parse scenario spec: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{Step, parse_spec};

    #[test]
    fn parse_spec_works_for_basic_file_flow() {
        let raw = r#"
name = "local-smoke"
timeout_ms = 1000

[[steps]]
kind = "create_dir"
path = "alpha"

[[steps]]
kind = "write_file"
path = "alpha/data.txt"
content = "hello"

[[steps]]
kind = "assert_exists"
path = "alpha/data.txt"
"#;
        let spec = parse_spec(raw).expect("spec should parse");
        assert_eq!(spec.name, "local-smoke");
        assert_eq!(spec.steps.len(), 3);
        match &spec.steps[1] {
            Step::WriteFile { path, content } => {
                assert_eq!(path, "alpha/data.txt");
                assert_eq!(content, "hello");
            }
            _ => panic!("unexpected step variant"),
        }
    }
}

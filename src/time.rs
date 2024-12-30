use chrono::prelude::*;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::{bad, Outcome};

const DEFAULT_FORMAT: &str = "%T";

/// Function to get the current timestamp as a formatted string
pub fn stamp(fmt: Option<&str>) -> String {
    let now: DateTime<Local> = Local::now();
    now.format(fmt.unwrap_or(DEFAULT_FORMAT)).to_string()
}

#[derive(Debug, PartialEq)]
struct LastSync {
    timestamp: String,
    cycle: u32,
}

#[allow(dead_code)]
impl LastSync {
    pub fn new() -> Self {
        LastSync {
            timestamp: stamp(None),
            cycle: 0,
        }
    }
    pub fn from(timestamp: String, cycle: u32) -> Self {
        LastSync { timestamp, cycle }
    }
}

/// Function to read LastSync from a given file path
#[allow(dead_code)]
fn read_last_sync<S>(path: &S) -> Outcome<LastSync>
where
    S: AsRef<std::ffi::OsStr> + AsRef<Path>,
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let timestamp = match lines.next() {
        Some(Ok(line)) => line.trim().to_string(),
        _ => return bad!("Missing timestamp"),
    };

    let cycle_str = match lines.next() {
        Some(Ok(line)) => line.trim().to_string(),
        _ => "0".to_string(), // Default cycle if not present
    };

    let cycle = cycle_str.parse::<u32>().unwrap_or(0);

    Ok(LastSync::from(timestamp, cycle))
}

/// Function to write LastSync to a given file path
#[allow(dead_code)]
fn write_last_sync<S>(path: &S, last_sync: &LastSync) -> Outcome<()>
where
    S: AsRef<std::ffi::OsStr> + AsRef<Path>,
{
    let file = File::create(path)?;
    let mut writer = io::BufWriter::new(file);
    writeln!(writer, "{}", last_sync.timestamp)?;
    writeln!(writer, "{}", last_sync.cycle)?;
    Ok(writer.flush()?)
}

/// Function to read 'lastsync' or create it if it doesn't exist
#[allow(dead_code)]
fn last_sync<S>(path: &S) -> Outcome<LastSync>
where
    S: AsRef<std::ffi::OsStr> + AsRef<Path> + ?Sized,
{
    let path = PathBuf::from(path);
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let last_sync = LastSync::new();
        write_last_sync(&path, &last_sync)?;
        return Ok(last_sync);
    }
    read_last_sync(&path) // previously ran
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn get_temp_file_path(filename: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(filename);
        path
    }

    #[test]
    fn check_stamp() {
        let current_timestamp = stamp(None);
        println!("Current Timestamp: {}", current_timestamp);
        assert!(!current_timestamp.is_empty());
        // Optionally, assert that the timestamp matches the expected format using regex
        // This requires the `regex` crate. If you prefer not to use external crates, you can skip this.
    }

    #[test]
    fn check_last_sync() {
        let temp_path = get_temp_file_path("test_lastsync");
        if temp_path.exists() {
            fs::remove_file(&temp_path).expect("Failed to remove existing temporary lastsync file");
        }
        let temp_path_str = temp_path
            .to_str()
            .expect("Failed to convert PathBuf to &str");
        let ls = last_sync(temp_path_str); // should create the file using LastSync::new()
        assert!(ls.is_ok(), "last_sync() returned an error");

        let ls = ls.unwrap();
        assert_eq!(ls.cycle, 0);
        assert!(!ls.timestamp.is_empty());

        // Now, modify the file to have a different timestamp and cycle
        {
            let mut file =
                File::create(&temp_path).expect("Failed to create temporary lastsync file");
            writeln!(file, "2024-04-01 12:00:00").expect("Failed to write timestamp");
            writeln!(file, "5").expect("Failed to write cycle");
        }

        // Call last_sync again, which should read the updated values
        let ls_updated = last_sync(temp_path_str);
        assert!(
            ls_updated.is_ok(),
            "last_sync() returned an error after update"
        );

        let ls_updated = ls_updated.unwrap();
        println!("Updated Last Sync: {:?}", ls_updated);
        assert_eq!(ls_updated.cycle, 5);
        assert_eq!(ls_updated.timestamp, "2024-04-01 12:00:00");
        fs::remove_file(&temp_path).expect("Failed to remove temporary lastsync file");
    }
}

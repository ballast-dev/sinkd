//! Shared daemon bootstrap: logging paths and [`DaemonType`]. Role-specific structs live in the
//! `sinkd` / `sinkd-srv` crates.

use std::{fmt, fs, path::Path, path::PathBuf};

use crate::{config, fancy, outcome::Outcome};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum DaemonType {
    UnixClient,
    UnixServer,
    WindowsClient,
    WindowsServer,
}

#[derive(Clone, Debug)]
pub struct SharedDaemonParams {
    pub daemon_type: DaemonType,
    pub verbosity: u8,
    pub debug: u8,
    pub log_path: PathBuf,
}

impl fmt::Display for SharedDaemonParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&fancy::format(
            &format!(
                r"🎨 SharedDaemonParams 🔍
daemon_type:{:?}
verbosity:{}
debug:{}
log_path:{}
",
                self.daemon_type,
                self.verbosity,
                self.debug,
                self.log_path.display(),
            ),
            fancy::Attrs::Bold,
            fancy::Colors::Yellow,
        ))
    }
}

fn log_base_dir(debug: u8) -> &'static Path {
    if debug >= 1 {
        Path::new("/tmp/sinkd")
    } else {
        Path::new("/var/log/sinkd")
    }
}

pub fn create_log_dir(debug: u8) -> Outcome<()> {
    let path = log_base_dir(debug);
    if path.exists() {
        return Ok(());
    }
    if debug == 0 && !config::have_permissions() {
        return bad!("Need elevated permissions to create {}", path.display());
    }
    match fs::create_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) => bad!("Unable to create '{}'  {}", path.display(), e),
    }
}

#[must_use]
pub fn get_log_path(debug: u8, daemon_type: DaemonType) -> PathBuf {
    let file = match daemon_type {
        DaemonType::UnixClient | DaemonType::WindowsClient => "client.log",
        DaemonType::UnixServer | DaemonType::WindowsServer => "server.log",
    };
    log_base_dir(debug).join(file)
}

//! Client-side backup directories for behind-pull (`behind_backups/N`).

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use crate::outcome::Outcome;

const BEHIND_BACKUPS: &str = "behind_backups";

fn is_decimal_dir(name: &OsStr) -> Option<u64> {
    let s = name.to_str()?;
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Returns absolute `…/behind_backups/N` with smallest unused `N` (from 0).
pub fn next_behind_backup_dir(client_state_dir: &Path) -> Outcome<PathBuf> {
    let behind = client_state_dir.join(BEHIND_BACKUPS);
    fs::create_dir_all(&behind)
        .map_err(|e| format!("behind_backups '{}': {e}", behind.display()))?;

    let mut used = std::collections::BTreeSet::<u64>::new();
    for entry in
        fs::read_dir(&behind).map_err(|e| format!("read_dir '{}': {e}", behind.display()))?
    {
        let entry = entry.map_err(|e| format!("read_dir entry '{}': {e}", behind.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("file_type '{}': {e}", entry.path().display()))?;
        if !file_type.is_dir() {
            continue;
        }
        if let Some(n) = is_decimal_dir(&entry.file_name()) {
            used.insert(n);
        }
    }

    let mut n: u64 = 0;
    while used.contains(&n) {
        n = n.saturating_add(1);
    }

    let run = behind.join(n.to_string());
    fs::create_dir(&run).map_err(|e| format!("create backup run '{}': {e}", run.display()))?;
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_behind_backup_dir_starts_at_zero() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = next_behind_backup_dir(tmp.path()).expect("first");
        assert!(p.ends_with("behind_backups/0") || p.ends_with("behind_backups\\0"));
        assert!(p.is_dir());
    }

    #[test]
    fn next_behind_backup_dir_skips_used() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tmp.path().join("behind_backups/0")).expect("mkdir");
        fs::create_dir_all(tmp.path().join("behind_backups/2")).expect("mkdir");
        let p = next_behind_backup_dir(tmp.path()).expect("next");
        assert!(p.ends_with("behind_backups/1") || p.ends_with("behind_backups\\1"));
    }

    #[test]
    fn next_behind_backup_dir_ignores_non_numeric_siblings() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(tmp.path().join("behind_backups/notes")).expect("mkdir");
        let p = next_behind_backup_dir(tmp.path()).expect("next");
        assert!(p.ends_with("behind_backups/0") || p.ends_with("behind_backups\\0"));
    }
}

use log::{debug, error};

use std::{ffi::OsStr, path::Path, process::Command};

use crate::config::ResolvedRsyncConfig;
use crate::outcome::Outcome;

pub fn rsync<P>(
    srcs: &[P],
    dest: &P,
    rsync_cfg: &ResolvedRsyncConfig,
    backup_dir: Option<&Path>,
) -> Outcome<()>
where
    P: AsRef<OsStr> + AsRef<Path> + std::fmt::Debug,
{
    if crate::test_hooks::env_flag_true("SINKD_TEST_RSYNC_FAIL") {
        error!("rsync test hook: forced failure");
        return bad!("rsync test hook: forced failure");
    }
    if let Some(delay_ms) = crate::test_hooks::env_u64("SINKD_TEST_RSYNC_DELAY_MS") {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }

    let mut cmd = Command::new("rsync");
    cmd.args(build_pull_args(rsync_cfg, backup_dir))
        .args(srcs)
        .arg(dest);

    let mut child = match cmd.spawn() {
        Err(e) => {
            error!("rsync spawn error: {e:#?}");
            return bad!("rsync spawn failed: {e}");
        }
        Ok(c) => c,
    };

    let status = match child.wait() {
        Err(e) => {
            error!("rsync wait error: {e:#?}");
            return bad!("rsync wait failed: {e}");
        }
        Ok(s) => s,
    };

    if !status.success() {
        error!("rsync exited with status {status}");
        return bad!("rsync failed with status {status}");
    }

    debug!("\u{1f6b0} rsync {srcs:#?} {dest:#?} backup:{backup_dir:?} \u{1f919}");
    Ok(())
}

/// Directory → directory sync (no `-R`). Used when the client reconciles from the
/// server's mirror tree during `NotReady(Behind)` — the server layout is
/// `{server_sync_root}/watch_user/...`, not the client's absolute anchor paths.
pub fn rsync_behind_mirror_sync(
    src_dir: &str,
    dest_dir: &str,
    rsync_cfg: &ResolvedRsyncConfig,
    backup_dir: Option<&Path>,
) -> Outcome<()> {
    if crate::test_hooks::env_flag_true("SINKD_TEST_RSYNC_FAIL") {
        error!("rsync test hook: forced failure");
        return bad!("rsync test hook: forced failure");
    }
    if let Some(delay_ms) = crate::test_hooks::env_u64("SINKD_TEST_RSYNC_DELAY_MS") {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }

    let mut cmd = Command::new("rsync");
    cmd.args(build_behind_pull_args(rsync_cfg, backup_dir))
        .arg(src_dir)
        .arg(dest_dir);

    let mut child = match cmd.spawn() {
        Err(e) => {
            error!("rsync spawn error: {e:#?}");
            return bad!("rsync spawn failed: {e}");
        }
        Ok(c) => c,
    };

    let status = match child.wait() {
        Err(e) => {
            error!("rsync wait error: {e:#?}");
            return bad!("rsync wait failed: {e}");
        }
        Ok(s) => s,
    };

    if !status.success() {
        error!("rsync exited with status {status}");
        return bad!("rsync failed with status {status}");
    }

    debug!("\u{1f6b0} rsync behind {src_dir} -> {dest_dir} backup:{backup_dir:?} \u{1f919}");
    Ok(())
}

#[must_use]
pub fn build_args(rsync_cfg: &ResolvedRsyncConfig) -> Vec<String> {
    let mut args = vec!["-atR".to_string(), "--delete".to_string()];
    append_rsync_option_flags(&mut args, rsync_cfg);
    args
}

#[must_use]
fn build_behind_pull_args(
    rsync_cfg: &ResolvedRsyncConfig,
    backup_dir: Option<&Path>,
) -> Vec<String> {
    // No `--delete`: when the server mirror for this anchor is still empty or lagging
    // (e.g. a second client joins while global generation is already ahead), a delete
    // pass would wipe the client's pending tree before the first successful push lands.
    let mut args = vec!["-a".to_string()];
    append_rsync_option_flags(&mut args, rsync_cfg);
    if let Some(dir) = backup_dir {
        args.push("--backup".to_string());
        args.push(format!(
            "--backup-dir={}",
            dir.as_os_str().to_string_lossy()
        ));
    }
    args
}

fn append_rsync_option_flags(args: &mut Vec<String>, rsync_cfg: &ResolvedRsyncConfig) {
    if rsync_cfg.checksum {
        args.push("--checksum".to_string());
    }
    if rsync_cfg.compress {
        args.push("--compress".to_string());
    }
    if let Some(bwlimit) = &rsync_cfg.bwlimit {
        args.push(format!("--bwlimit={bwlimit}"));
    }
    if rsync_cfg.partial {
        args.push("--partial".to_string());
    }
    if rsync_cfg.delete_excluded {
        args.push("--delete-excluded".to_string());
    }
    if let Some(max_size) = &rsync_cfg.max_size {
        args.push(format!("--max-size={max_size}"));
    }
    if let Some(min_size) = &rsync_cfg.min_size {
        args.push(format!("--min-size={min_size}"));
    }
    if rsync_cfg.ignore_existing {
        args.push("--ignore-existing".to_string());
    }
    if rsync_cfg.size_only {
        args.push("--size-only".to_string());
    }
    if rsync_cfg.stats {
        args.push("--stats".to_string());
    }
}

/// Full rsync argv for a pull (baseline flags plus optional `--backup` / `--backup-dir`).
#[must_use]
pub fn build_pull_args(rsync_cfg: &ResolvedRsyncConfig, backup_dir: Option<&Path>) -> Vec<String> {
    let mut args = build_args(rsync_cfg);
    if let Some(dir) = backup_dir {
        args.push("--backup".to_string());
        args.push(format!(
            "--backup-dir={}",
            dir.as_os_str().to_string_lossy()
        ));
    }
    args
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::config::ResolvedRsyncConfig;

    use super::{build_args, build_pull_args};

    #[test]
    fn build_args_keeps_baseline_defaults() {
        let args = build_args(&ResolvedRsyncConfig::default());
        assert_eq!(args, vec!["-atR", "--delete"]);
    }

    #[test]
    fn build_args_appends_allowlisted_flags() {
        let cfg = ResolvedRsyncConfig {
            checksum: true,
            compress: true,
            bwlimit: Some("2m".to_string()),
            partial: true,
            delete_excluded: true,
            max_size: Some("10m".to_string()),
            min_size: Some("1k".to_string()),
            ignore_existing: true,
            size_only: true,
            stats: true,
        };
        let args = build_args(&cfg);
        assert_eq!(
            args,
            vec![
                "-atR",
                "--delete",
                "--checksum",
                "--compress",
                "--bwlimit=2m",
                "--partial",
                "--delete-excluded",
                "--max-size=10m",
                "--min-size=1k",
                "--ignore-existing",
                "--size-only",
                "--stats",
            ]
        );
    }

    #[test]
    fn build_pull_args_matches_build_args_without_backup() {
        let cfg = ResolvedRsyncConfig::default();
        assert_eq!(build_pull_args(&cfg, None), build_args(&cfg));
    }

    #[test]
    fn build_pull_args_appends_backup_flags_with_absolute_dir() {
        let cfg = ResolvedRsyncConfig::default();
        let backup = Path::new("/tmp/sinkd/client/behind_backups/0");
        let args = build_pull_args(&cfg, Some(backup));
        assert!(
            args.ends_with(&[
                "--backup".to_string(),
                "--backup-dir=/tmp/sinkd/client/behind_backups/0".to_string(),
            ]),
            "args={args:?}"
        );
    }
}

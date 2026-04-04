use log::{debug, error};

use std::{ffi::OsStr, path::Path, process::Command};

use crate::config::ResolvedRsyncConfig;

pub fn rsync<P>(srcs: &[P], dest: &P, rsync_cfg: &ResolvedRsyncConfig)
where
    P: AsRef<OsStr> + AsRef<Path> + std::fmt::Debug,
{
    if std::env::var("SINKD_TEST_RSYNC_FAIL")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    {
        error!("rsync test hook: forced failure");
        return;
    }
    if let Some(delay_ms) = std::env::var("SINKD_TEST_RSYNC_DELAY_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }

    let mut cmd = Command::new("rsync");

    cmd.args(build_args(rsync_cfg)).args(srcs).arg(dest);

    match cmd.spawn() {
        Err(x) => error!("{x:#?}"),
        Ok(_) => debug!("\u{1f6b0} rsync {srcs:#?} {dest:#?} \u{1f919}"),
    }
}

pub fn build_args(rsync_cfg: &ResolvedRsyncConfig) -> Vec<String> {
    let mut args = vec!["-atR".to_string(), "--delete".to_string()];
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
    args
}

#[cfg(test)]
mod tests {
    use crate::config::ResolvedRsyncConfig;

    use super::build_args;

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
}

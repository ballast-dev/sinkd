//! Shared template-render logic for `sinkd client init` and `sinkd server init`.
//!
//! Each `init` subcommand scaffolds a TOML config from a template — disk-first
//! (`/usr/share/sinkd/*.conf` installed by the package) with embedded fallbacks
//! baked into the binary via [`include_str!`]. Placeholders are `{{name}}` style
//! and substituted in-place.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::outcome::Outcome;
use log::info;

/// Embedded system-config template (used when no on-disk template is found).
pub const SYSTEM_TEMPLATE: &str = include_str!("../../cfg/templates/sinkd.system.conf.tmpl");
/// Embedded user-config template (used when no on-disk template is found).
pub const USER_TEMPLATE: &str = include_str!("../../cfg/templates/sinkd.user.conf.tmpl");

/// On-disk locations populated by the Debian package; checked before falling
/// back to the embedded copies.
pub const SYSTEM_TEMPLATE_DISK: &str = "/usr/share/sinkd/sinkd.conf";
pub const USER_TEMPLATE_DISK: &str = "/usr/share/sinkd/sinkd.user.conf";

/// Inputs for [`render`].
pub struct InitOptions<'a> {
    pub target_path: PathBuf,
    pub template_disk: Option<&'a Path>,
    pub template_embedded: &'static str,
    pub substitutions: &'a [(&'a str, String)],
    pub force: bool,
}

/// Render a config to disk: pick the template (disk preferred), substitute
/// `{{key}}` placeholders, and atomically write to `target_path`. Refuses to
/// overwrite an existing file unless `force` is set.
pub fn render(opts: &InitOptions<'_>) -> Outcome<()> {
    if opts.target_path.exists() && !opts.force {
        info!(
            "init: leaving existing config at {} (pass --force to overwrite)",
            opts.target_path.display()
        );
        return Ok(());
    }

    let template = load_template(opts.template_disk, opts.template_embedded)?;
    let rendered = substitute(&template, opts.substitutions);

    if let Some(parent) = opts.target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("init: create parent '{}': {e}", parent.display()))?;
    }

    write_atomic(&opts.target_path, &rendered)?;
    info!("init: wrote {}", opts.target_path.display());
    Ok(())
}

fn load_template(disk: Option<&Path>, embedded: &'static str) -> Outcome<String> {
    if let Some(path) = disk {
        if path.exists() {
            return fs::read_to_string(path)
                .map_err(|e| format!("init: read template '{}': {e}", path.display()).into());
        }
    }
    Ok(embedded.to_string())
}

/// Replace every occurrence of `{{key}}` with its substitution value. Unknown
/// placeholders are left as-is so a malformed template surfaces as a literal
/// `{{...}}` in the rendered output rather than silently dropping content.
#[must_use]
pub fn substitute(template: &str, subs: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (key, value) in subs {
        let needle = format!("{{{{{key}}}}}");
        out = out.replace(&needle, value);
    }
    out
}

fn write_atomic(target: &Path, contents: &str) -> Outcome<()> {
    let tmp = target.with_extension("tmp.init");
    fs::write(&tmp, contents)
        .map_err(|e| format!("init: write temp '{}': {e}", tmp.display()))?;
    fs::rename(&tmp, target)
        .map_err(|e| format!("init: rename '{}' -> '{}': {e}", tmp.display(), target.display()))?;
    Ok(())
}

/// Format a TOML string array literal body — `"a", "b", "c"` — used by the
/// system template's `{{users}}` placeholder.
#[must_use]
pub fn toml_string_array_body(items: &[String]) -> String {
    items
        .iter()
        .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_replaces_known_placeholders() {
        let out = substitute(
            "addr={{server_addr}} user={{user}}",
            &[
                ("server_addr", "host.example".into()),
                ("user", "alice".into()),
            ],
        );
        assert_eq!(out, "addr=host.example user=alice");
    }

    #[test]
    fn substitute_leaves_unknown_placeholders_untouched() {
        let out = substitute("a={{a}} b={{b}}", &[("a", "1".into())]);
        assert_eq!(out, "a=1 b={{b}}");
    }

    #[test]
    fn toml_string_array_body_quotes_and_escapes() {
        let body =
            toml_string_array_body(&["alice".into(), "bo\"b".into(), "c\\arol".into()]);
        assert_eq!(body, r#""alice", "bo\"b", "c\\arol""#);
    }

    #[test]
    fn render_writes_when_target_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("nested/sinkd.conf");
        render(&InitOptions {
            target_path: target.clone(),
            template_disk: None,
            template_embedded: "addr={{addr}}\n",
            substitutions: &[("addr", "127.0.0.1".into())],
            force: false,
        })
        .expect("render");
        assert_eq!(
            std::fs::read_to_string(&target).expect("read"),
            "addr=127.0.0.1\n"
        );
    }

    #[test]
    fn render_skips_when_target_exists_without_force() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("sinkd.conf");
        std::fs::write(&target, "preserved").expect("seed");
        render(&InitOptions {
            target_path: target.clone(),
            template_disk: None,
            template_embedded: "fresh",
            substitutions: &[],
            force: false,
        })
        .expect("render");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "preserved");
    }

    #[test]
    fn render_overwrites_with_force() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("sinkd.conf");
        std::fs::write(&target, "stale").expect("seed");
        render(&InitOptions {
            target_path: target.clone(),
            template_disk: None,
            template_embedded: "fresh",
            substitutions: &[],
            force: true,
        })
        .expect("render");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "fresh");
    }

    #[test]
    fn render_prefers_disk_template_over_embedded() {
        let dir = tempfile::tempdir().expect("tempdir");
        let disk = dir.path().join("disk.tmpl");
        std::fs::write(&disk, "from-disk={{x}}").expect("seed disk");
        let target = dir.path().join("out.conf");
        render(&InitOptions {
            target_path: target.clone(),
            template_disk: Some(&disk),
            template_embedded: "from-embedded={{x}}",
            substitutions: &[("x", "1".into())],
            force: false,
        })
        .expect("render");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "from-disk=1");
    }
}

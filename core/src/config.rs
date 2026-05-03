//! **Two configuration surfaces (by design):**
//! - **Client** — system TOML (`/etc/sinkd.conf` or `--sys-cfg`) plus per-user TOML files; consumed by
//!   client crate via paths passed to [`crate::config::get_for_client_paths`]. `server_addr` in the
//!   system file is the sync target/description for clients (see also client-side `_srv_addr` note in
//!   client daemon init).
//! - **Server** — runtime sync root under `/srv/sinkd` (or debug path) and `generation_state.toml` there; the
//!   server does **not** load client TOML anchor lists for queue/dedup logic.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

macro_rules! reject_unsupported_rsync_fields {
    ($cfg:expr, $($field:ident),+ $(,)?) => {
        $(
            if $cfg.$field.is_some() {
                return Err(format!("unsupported rsync flag `{}` in config", stringify!($field)));
            }
        )+
    };
}

use crate::outcome::Outcome;
use log::{error, warn};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResolvedRsyncConfig {
    pub checksum: bool,
    pub compress: bool,
    pub bwlimit: Option<String>,
    pub partial: bool,
    pub delete_excluded: bool,
    pub max_size: Option<String>,
    pub min_size: Option<String>,
    pub ignore_existing: bool,
    pub size_only: bool,
    pub stats: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RsyncConfig {
    pub checksum: Option<bool>,
    pub compress: Option<bool>,
    pub bwlimit: Option<String>,
    pub partial: Option<bool>,
    pub delete_excluded: Option<bool>,
    pub max_size: Option<String>,
    pub min_size: Option<String>,
    pub ignore_existing: Option<bool>,
    pub size_only: Option<bool>,
    pub stats: Option<bool>,
    pub owner: Option<toml::Value>,
    pub group: Option<toml::Value>,
    pub devices: Option<toml::Value>,
    pub specials: Option<toml::Value>,
    pub super_user: Option<toml::Value>,
    pub fake_super: Option<toml::Value>,
    pub rsh: Option<toml::Value>,
    pub address: Option<toml::Value>,
    pub port: Option<toml::Value>,
    pub sockopts: Option<toml::Value>,
    pub remove_source_files: Option<toml::Value>,
    pub files_from: Option<toml::Value>,
    pub include_from: Option<toml::Value>,
    pub exclude_from: Option<toml::Value>,
    pub from0: Option<toml::Value>,
    pub usermap: Option<toml::Value>,
    pub groupmap: Option<toml::Value>,
    pub chown: Option<toml::Value>,
    pub chmod: Option<toml::Value>,
}

impl RsyncConfig {
    fn validate(&self) -> Result<(), String> {
        reject_unsupported_rsync_fields!(
            self,
            owner,
            group,
            devices,
            specials,
            super_user,
            fake_super,
            rsh,
            address,
            port,
            sockopts,
            remove_source_files,
            files_from,
            include_from,
            exclude_from,
            from0,
            usermap,
            groupmap,
            chown,
            chmod,
        );
        Ok(())
    }

    fn merge_over(&self, base: &ResolvedRsyncConfig) -> ResolvedRsyncConfig {
        ResolvedRsyncConfig {
            checksum: self.checksum.unwrap_or(base.checksum),
            compress: self.compress.unwrap_or(base.compress),
            bwlimit: self.bwlimit.clone().or_else(|| base.bwlimit.clone()),
            partial: self.partial.unwrap_or(base.partial),
            delete_excluded: self.delete_excluded.unwrap_or(base.delete_excluded),
            max_size: self.max_size.clone().or_else(|| base.max_size.clone()),
            min_size: self.min_size.clone().or_else(|| base.min_size.clone()),
            ignore_existing: self.ignore_existing.unwrap_or(base.ignore_existing),
            size_only: self.size_only.unwrap_or(base.size_only),
            stats: self.stats.unwrap_or(base.stats),
        }
    }
}

// these are serially parsable
#[derive(Debug, Serialize, Deserialize)]
pub struct Anchor {
    pub path: PathBuf,
    interval: Option<u64>,
    excludes: Option<Vec<String>>,
    rsync: Option<RsyncConfig>,
    rsync_checksum: Option<bool>,
    rsync_compress: Option<bool>,
    rsync_bwlimit: Option<String>,
    rsync_partial: Option<bool>,
    rsync_delete_excluded: Option<bool>,
    rsync_max_size: Option<String>,
    rsync_min_size: Option<String>,
    rsync_ignore_existing: Option<bool>,
    rsync_size_only: Option<bool>,
    rsync_stats: Option<bool>,
}

impl Anchor {
    #[must_use]
    pub fn with_path(path: PathBuf) -> Self {
        Anchor {
            path,
            interval: None,
            excludes: None,
            rsync: None,
            rsync_checksum: None,
            rsync_compress: None,
            rsync_bwlimit: None,
            rsync_partial: None,
            rsync_delete_excluded: None,
            rsync_max_size: None,
            rsync_min_size: None,
            rsync_ignore_existing: None,
            rsync_size_only: None,
            rsync_stats: None,
        }
    }

    fn rsync_override(&self) -> RsyncConfig {
        let mut cfg = self.rsync.clone().unwrap_or_default();
        if self.rsync_checksum.is_some() {
            cfg.checksum = self.rsync_checksum;
        }
        if self.rsync_compress.is_some() {
            cfg.compress = self.rsync_compress;
        }
        if self.rsync_bwlimit.is_some() {
            cfg.bwlimit.clone_from(&self.rsync_bwlimit);
        }
        if self.rsync_partial.is_some() {
            cfg.partial = self.rsync_partial;
        }
        if self.rsync_delete_excluded.is_some() {
            cfg.delete_excluded = self.rsync_delete_excluded;
        }
        if self.rsync_max_size.is_some() {
            cfg.max_size.clone_from(&self.rsync_max_size);
        }
        if self.rsync_min_size.is_some() {
            cfg.min_size.clone_from(&self.rsync_min_size);
        }
        if self.rsync_ignore_existing.is_some() {
            cfg.ignore_existing = self.rsync_ignore_existing;
        }
        if self.rsync_size_only.is_some() {
            cfg.size_only = self.rsync_size_only;
        }
        if self.rsync_stats.is_some() {
            cfg.stats = self.rsync_stats;
        }
        cfg
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SysConfig {
    pub server_addr: String,
    /// On the sync server, the directory where mirrored client trees live (`/srv/sinkd/...`).
    /// Used when [`Self::server_addr`] is a hostname (remote `rsync`). Ignored when
    /// `server_addr` is itself an absolute path (local/shared-volume mirror).
    #[serde(default)]
    pub server_sync_root: Option<String>,
    pub users: Vec<String>,
    pub anchors: Option<Vec<Anchor>>,
    pub rsync: Option<RsyncConfig>,
}

pub fn load_system_config_file(path: &Path) -> Outcome<SysConfig> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("cannot read system config {}: {e}", path.display()))?;
    Ok(toml::from_str(&raw)
        .map_err(|e| format!("cannot parse system config {}: {e}", path.display()))?)
}

pub fn save_system_config_file(path: &Path, cfg: &SysConfig) -> Outcome<()> {
    let serialized =
        toml::to_string_pretty(cfg).map_err(|e| format!("cannot serialize system config: {e}"))?;
    fs::write(path, serialized)?;
    Ok(())
}

pub fn load_user_config_file(path: &Path) -> Outcome<UserConfig> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("cannot read user config {}: {e}", path.display()))?;
    Ok(toml::from_str(&raw)
        .map_err(|e| format!("cannot parse user config {}: {e}", path.display()))?)
}

pub fn save_user_config_file(path: &Path, cfg: &UserConfig) -> Outcome<()> {
    let serialized =
        toml::to_string_pretty(cfg).map_err(|e| format!("cannot serialize user config: {e}"))?;
    fs::write(path, serialized)?;
    Ok(())
}

impl SysConfig {
    fn new() -> SysConfig {
        SysConfig {
            server_addr: String::new(),
            server_sync_root: None,
            users: Vec::new(),
            anchors: Some(Vec::new()),
            rsync: None,
        }
    }
}

/// Path on the server (or local mirror mount) that corresponds to the client's
/// anchor root after `rsync -aR` from the client (`/srv/sinkd` + `/watch_alice` → `/srv/sinkd/watch_alice`).
#[must_use]
pub fn server_mirror_path(anchor: &Path, server_mirror_root: &Path) -> PathBuf {
    let tail = anchor.strip_prefix("/").unwrap_or(anchor);
    server_mirror_root.join(tail)
}

fn resolve_server_mirror_root(sys: &SysConfig) -> PathBuf {
    let sync_root = sys
        .server_sync_root
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("/srv/sinkd");
    if sys.server_addr.starts_with('/') {
        PathBuf::from(&sys.server_addr)
    } else {
        PathBuf::from(sync_root)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserConfig {
    pub anchors: Vec<Anchor>,
    pub rsync: Option<RsyncConfig>,
}

#[allow(dead_code)]
#[derive(PartialEq)]
enum ParseError {
    FileNotFound,
    InvalidSyntax(String),
    NoUserFound,
}

struct ConfigParser {
    sys: SysConfig,
    users: HashMap<PathBuf, UserConfig>,
}

#[doc = "don't need a class to operate"]
impl ConfigParser {
    // system level config that will house user based configs
    fn new() -> ConfigParser {
        ConfigParser {
            sys: SysConfig::new(),
            users: HashMap::new(),
        }
    }

    fn parse_configs_paths(
        &mut self,
        system_config: &Path,
        user_configs: &[PathBuf],
    ) -> Outcome<()> {
        if let Err(e) = self.parse_sys_config(system_config) {
            match e {
                ParseError::InvalidSyntax(syn) => {
                    return bad!("Invalid sytax in '{}': {}", system_config.display(), syn);
                }
                ParseError::FileNotFound => {
                    return bad!("File not found: '{}'", system_config.display());
                }
                ParseError::NoUserFound => return bad!("No user found"),
            }
        }

        if let Err(ParseError::NoUserFound) = self.parse_user_configs(user_configs) {
            warn!("No user was loaded into sinkd, using only system configs");
        }
        Ok(())
    }

    fn parse_sys_config(&mut self, sys_config: &Path) -> Result<(), ParseError> {
        match fs::read_to_string(sys_config) {
            Err(_) => Err(ParseError::FileNotFound),
            Ok(output) => match toml::from_str(&output) {
                Err(error) => Err(ParseError::InvalidSyntax(error.to_string())),
                Ok(toml_parsed) => {
                    self.sys = toml_parsed; // NOTE: converted into Rust via serde lib
                    if let Some(rsync) = &self.sys.rsync {
                        rsync.validate().map_err(ParseError::InvalidSyntax)?;
                    }
                    if let Some(anchors) = &self.sys.anchors {
                        for anchor in anchors {
                            anchor
                                .rsync_override()
                                .validate()
                                .map_err(ParseError::InvalidSyntax)?;
                        }
                    }
                    Ok(())
                }
            },
        }
    }

    fn parse_user_configs(&mut self, user_configs: &[PathBuf]) -> Result<(), ParseError> {
        for user_config in user_configs {
            match ConfigParser::get_user_config(user_config.as_path()) {
                Ok(usr_cfg) => {
                    let _ = &self.users.insert(user_config.clone(), usr_cfg);
                }
                Err(error) => match error {
                    ParseError::FileNotFound => {
                        error!("File not found: {}", user_config.display());
                    }
                    ParseError::InvalidSyntax(syntax) => {
                        error!("Invalid syntax in: {}: {}", user_config.display(), syntax);
                    }
                    ParseError::NoUserFound => (),
                },
            }
        }

        if self.users.is_empty() {
            return Err(ParseError::NoUserFound);
        }
        Ok(())
    }

    fn get_user_config(user_config: &Path) -> Result<UserConfig, ParseError> {
        match fs::read_to_string(user_config) {
            Err(_) => Err(ParseError::FileNotFound),
            Ok(output) => match toml::from_str(&output) {
                Err(error) => Err(ParseError::InvalidSyntax(error.to_string())),
                Ok(toml_parsed) => {
                    let user_config: UserConfig = toml_parsed;
                    if let Some(rsync) = &user_config.rsync {
                        rsync.validate().map_err(ParseError::InvalidSyntax)?;
                    }
                    for anchor in &user_config.anchors {
                        anchor
                            .rsync_override()
                            .validate()
                            .map_err(ParseError::InvalidSyntax)?;
                    }
                    Ok(user_config)
                }
            },
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Inode {
    pub excludes: Vec<String>, // holds wildcards
    pub interval: Duration,
    pub last_event: Instant,
    pub event: bool,
    pub rsync: ResolvedRsyncConfig,
}

pub type InodeMap = HashMap<PathBuf, Inode>;

pub fn get_for_client_paths(
    system_config: &Path,
    user_configs: &[PathBuf],
) -> Outcome<(String, PathBuf, InodeMap)> {
    let mut parser = ConfigParser::new();
    parser.parse_configs_paths(system_config, user_configs)?;

    let mut inode_map: InodeMap = HashMap::new();
    let sys_rsync = parser
        .sys
        .rsync
        .as_ref()
        .map_or_else(ResolvedRsyncConfig::default, |cfg| {
            cfg.merge_over(&ResolvedRsyncConfig::default())
        });

    for cfg in parser.users.values() {
        let user_rsync = cfg.rsync.as_ref().map_or_else(
            || sys_rsync.clone(),
            |override_cfg| override_cfg.merge_over(&sys_rsync),
        );
        for anchor in &cfg.anchors {
            // let excludes = anchor.excludes.is_some().or()
            let resolved_rsync = anchor.rsync_override().merge_over(&user_rsync);
            inode_map.entry(anchor.path.clone()).or_insert(Inode {
                excludes: anchor.excludes.clone().unwrap_or(vec![]),
                interval: Duration::from_secs(anchor.interval.unwrap_or(5)),
                last_event: Instant::now(),
                event: false,
                rsync: resolved_rsync,
            });
        }
    }

    if let Some(anchors) = &parser.sys.anchors {
        for anchor in anchors {
            let resolved_rsync = anchor.rsync_override().merge_over(&sys_rsync);
            inode_map.entry(anchor.path.clone()).or_insert(Inode {
                excludes: anchor.excludes.clone().unwrap_or(vec![]),
                interval: Duration::from_secs(anchor.interval.unwrap_or(5)),
                last_event: Instant::now(),
                event: false,
                rsync: resolved_rsync,
            });
        }
    }
    let mirror_root = resolve_server_mirror_root(&parser.sys);
    Ok((parser.sys.server_addr, mirror_root, inode_map))
}

#[must_use]
pub fn have_permissions() -> bool {
    #[cfg(unix)]
    {
        // get effective user ID
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Security::GetTokenInformation;
        use windows::Win32::Security::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let process_handle = GetCurrentProcess();
            let mut token_handle: HANDLE = HANDLE::default();
            if OpenProcessToken(process_handle, TOKEN_QUERY, &raw mut token_handle).is_err() {
                return false;
            }
            // Check if the token has admin rights
            let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
            let mut return_length = 0;
            let size = u32::try_from(std::mem::size_of::<TOKEN_ELEVATION>()).unwrap_or(u32::MAX);
            if GetTokenInformation(
                token_handle,
                TokenElevation,
                // look away, interfacing with Windows is hacky even with windows crate
                Some((&raw mut elevation).cast()),
                size,
                &raw mut return_length,
            )
            .is_err()
            {
                return false;
            }

            elevation.TokenIsElevated != 0
        }
    }
}

#[cfg(unix)]
pub fn get_hostname() -> Outcome<String> {
    use libc::{c_char, sysconf, _SC_HOST_NAME_MAX};
    use std::ffi::CStr;

    unsafe {
        // Get the maximum hostname length
        let max_len = sysconf(_SC_HOST_NAME_MAX);
        if max_len == -1 {
            return bad!("Failed to determine maximum hostname length");
        }

        let mut buffer =
            vec![0u8; usize::try_from(max_len).map_err(|_| "Invalid hostname buffer size")?];
        let ptr = buffer.as_mut_ptr().cast::<c_char>();

        if libc::gethostname(
            ptr,
            usize::try_from(max_len).map_err(|_| "Invalid hostname buffer size")?,
        ) != 0
        {
            return bad!("Failed to retrieve hostname");
        }

        // Convert the hostname from C string to Rust string
        let cstr = CStr::from_ptr(ptr);
        match cstr.to_str() {
            Ok(s) => Ok(s.to_owned()),
            Err(e) => bad!("Failed to convert hostname to UTF-8: {}", e),
        }
    }
}

#[cfg(windows)]
pub fn get_hostname() -> Outcome<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::WindowsProgramming::{GetComputerNameW, MAX_COMPUTERNAME_LENGTH};

    let mut buffer = [0u16; MAX_COMPUTERNAME_LENGTH as usize + 1];
    let mut size = u32::try_from(buffer.len()).unwrap_or(u32::MAX);

    unsafe {
        let pwstr = windows::core::PWSTR(buffer.as_mut_ptr());
        if let Err(e) = GetComputerNameW(Some(pwstr), &raw mut size) {
            bad!("Failed to retrieve the hostname: {}", e)
        } else {
            let hostname = OsString::from_wide(&buffer[..size as usize]);
            Ok(hostname.to_string_lossy().into_owned())
        }
    }
}

pub fn get_username() -> Outcome<String> {
    if let Some(username) = std::env::var("USER")
        .ok()
        .or(std::env::var("USERNAME").ok())
    {
        return Ok(username);
    }
    bad!("USER not found")
}

// this will resolve all known paths, converts relative to absolute
pub fn resolve(path: &str) -> Outcome<PathBuf> {
    // NOTE: `~` is a shell expansion not handled by system calls
    if path.starts_with("~/") {
        let mut p = match std::env::var("HOME") {
            Ok(home_dir) => PathBuf::from(home_dir),
            Err(e) => {
                return bad!("HOME env var not defined: {}", e);
            }
        };
        let after_tilde = path
            .strip_prefix("~/")
            .ok_or_else(|| format!("internal: path was expected to start with '~/': {path}"))?;
        p.push(after_tilde);
        match p.canonicalize() {
            Ok(resolved) => Ok(resolved),
            Err(e) => bad!("{} '{}'", e, p.display()),
        }
    } else {
        match PathBuf::from(path).canonicalize() {
            Ok(resolved) => Ok(resolved),
            Err(e) => bad!("{} '{}'", e, path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Anchor, ResolvedRsyncConfig, RsyncConfig};

    #[test]
    fn rsync_config_rejects_unsupported_flags() {
        let cfg: RsyncConfig =
            toml::from_str("owner = true").expect("config with known but unsupported field parses");
        let err = cfg
            .validate()
            .expect_err("unsupported field should fail validation");
        assert!(err.contains("owner"));
    }

    #[test]
    fn rsync_config_rejects_unknown_fields() {
        let err = toml::from_str::<RsyncConfig>("made_up_flag = true")
            .expect_err("unknown field should be rejected");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn rsync_config_merge_applies_anchor_override_on_global_defaults() {
        let global: RsyncConfig = toml::from_str(
            r#"
            partial = true
            compress = false
            bwlimit = "8m"
            "#,
        )
        .expect("global rsync should parse");
        let anchor: RsyncConfig = toml::from_str(
            r#"
            compress = true
            max_size = "10m"
            "#,
        )
        .expect("anchor rsync should parse");

        let resolved_global = global.merge_over(&ResolvedRsyncConfig::default());
        let resolved_anchor = anchor.merge_over(&resolved_global);

        assert!(resolved_anchor.partial);
        assert!(resolved_anchor.compress);
        assert_eq!(resolved_anchor.bwlimit.as_deref(), Some("8m"));
        assert_eq!(resolved_anchor.max_size.as_deref(), Some("10m"));
    }

    #[test]
    fn anchor_flattened_rsync_override_is_parsed() {
        let anchor: Anchor = toml::from_str(
            r#"
            path = "/tmp/a"
            rsync_compress = true
            rsync_max_size = "10m"
            "#,
        )
        .expect("anchor with flattened rsync fields should parse");
        let cfg = anchor.rsync_override();
        assert_eq!(cfg.compress, Some(true));
        assert_eq!(cfg.max_size.as_deref(), Some("10m"));
    }
}

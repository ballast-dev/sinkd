//! Composed runtime parameters (shared logging + role-specific fields). Client commands use TOML paths
//! (`system_config`, `user_configs`); the server daemon uses only [`crate::server`]’s sync root and
//! persisted generation / client id state — see [`crate::config`] for the split between client and server configuration.

use clap::{parser::ValuesRef, ArgMatches};
use std::{
    fmt,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{config, fancy, outcome::Outcome};
use log::{debug, error};

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

#[derive(Clone, Debug)]
pub struct ClientParameters {
    pub shared: SharedDaemonParams,
    pub system_config: Arc<PathBuf>,
    pub user_configs: Arc<Vec<PathBuf>>,
    pub client_state_dir_override: Option<PathBuf>,
}

impl fmt::Display for ClientParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.shared)?;
        write!(
            f,
            "system_config:{}\nuser configs: [{}]\nclient_state_dir_override:{}\n",
            self.system_config.display(),
            self.user_configs
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.client_state_dir_override
                .as_ref()
                .map(|p| p.display().to_string())
                .as_deref()
                .unwrap_or("(default)")
        )
    }
}

#[derive(Clone, Debug)]
pub struct ServerParameters {
    pub shared: SharedDaemonParams,
}

impl fmt::Display for ServerParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.shared.fmt(f)
    }
}

#[derive(Clone, Debug)]
pub enum DaemonParameters {
    Client(ClientParameters),
    Server(ServerParameters),
}

impl fmt::Display for DaemonParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Client(c) => c.fmt(f),
            Self::Server(s) => s.fmt(f),
        }
    }
}

impl DaemonParameters {
    #[must_use]
    pub fn shared(&self) -> &SharedDaemonParams {
        match self {
            Self::Client(c) => &c.shared,
            Self::Server(s) => &s.shared,
        }
    }

    pub fn from_matches(matches: &ArgMatches) -> Outcome<Self> {
        let windows = matches.get_flag("windows-daemon");
        let (system_config, user_configs, daemon_type) = match matches.subcommand() {
            Some(("client", client_m)) => (
                client_m.get_one("system-config"),
                client_m.get_many("user-configs"),
                if windows {
                    DaemonType::WindowsClient
                } else {
                    DaemonType::UnixClient
                },
            ),
            _ => (
                None,
                None,
                if windows {
                    DaemonType::WindowsServer
                } else {
                    DaemonType::UnixServer
                },
            ),
        };

        let debug = matches.get_count("debug");
        create_log_dir(debug)?;

        let debug_level = match debug {
            1 | 2 => debug,
            d if d > 2 => {
                println!("debug only has two levels");
                2
            }
            _ => 0,
        };

        let verbosity = match (debug, matches.get_count("verbose")) {
            (d, _) if d > 0 => 4,
            (_, 0) => 2,
            (_, v) => v,
        };

        let shared = SharedDaemonParams {
            daemon_type,
            verbosity,
            debug: debug_level,
            log_path: get_log_path(debug, daemon_type),
        };

        let client_state_dir_override = matches
            .get_one::<String>("client-state-dir")
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty());

        let params = match daemon_type {
            DaemonType::UnixClient | DaemonType::WindowsClient => {
                Self::Client(ClientParameters {
                    shared,
                    system_config: resolve_system_config(system_config)?,
                    user_configs: resolve_user_configs(user_configs)?,
                    client_state_dir_override,
                })
            }
            DaemonType::UnixServer | DaemonType::WindowsServer => {
                Self::Server(ServerParameters { shared })
            }
        };

        if params.shared().debug > 0 {
            println!("{params}");
        }

        Ok(params)
    }
}

fn log_base_dir(debug: u8) -> &'static Path {
    if debug >= 1 {
        Path::new("/tmp/sinkd")
    } else {
        Path::new("/var/log/sinkd")
    }
}

fn create_log_dir(debug: u8) -> Outcome<()> {
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

fn get_log_path(debug: u8, daemon_type: DaemonType) -> PathBuf {
    let file = match daemon_type {
        DaemonType::UnixClient | DaemonType::WindowsClient => "client.log",
        DaemonType::UnixServer | DaemonType::WindowsServer => "server.log",
    };
    log_base_dir(debug).join(file)
}

fn resolve_system_config(system_config: Option<&String>) -> Outcome<Arc<PathBuf>> {
    let cfg_path: PathBuf;
    if let Some(sys_cfg) = system_config {
        debug!("resolve_system_config>> passed in: {sys_cfg}");
        match config::resolve(sys_cfg) {
            Ok(normalized) => {
                if normalized.is_dir() {
                    return bad!(
                        "{} is a directory not a file, aborting",
                        normalized.display()
                    );
                } else if normalized.exists() {
                    cfg_path = normalized;
                } else {
                    return bad!("{} does not exist", normalized.display());
                }
            }
            Err(e) => return bad!("system config path error: {}", e),
        }
    } else if cfg!(target_os = "macos") {
        cfg_path = PathBuf::from("/opt/sinkd/sinkd.conf");
    } else if cfg!(target_os = "windows") {
        cfg_path = PathBuf::from("/somepath/sinkd.conf");
    } else {
        cfg_path = PathBuf::from("/etc/sinkd.conf");
    }

    debug!("system config: {}", cfg_path.display());

    Ok(Arc::new(cfg_path))
}

pub fn resolve_user_configs(user_configs: Option<ValuesRef<String>>) -> Outcome<Arc<Vec<PathBuf>>> {
    let mut resolved_configs = Vec::<PathBuf>::new();

    if let Some(usr_cfgs) = user_configs {
        for cfg in usr_cfgs {
            let normalized = config::resolve(cfg)?;
            if normalized.is_dir() {
                return bad!(
                    "{} is a directory, not a file; aborting",
                    normalized.display()
                );
            }
            resolved_configs.push(normalized);
        }
    } else {
        let default_cfgs = vec!["~/.config/sinkd/sinkd.conf", "~/sinkd.conf"];

        for cfg in default_cfgs {
            match config::resolve(cfg) {
                Ok(resolved_user_config) => resolved_configs.push(resolved_user_config),
                Err(e) => error!("Unable to resolve {cfg}  {e}"),
            }
        }
    }

    debug!(
        "user configs: [{}]",
        resolved_configs
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(Arc::new(resolved_configs))
}

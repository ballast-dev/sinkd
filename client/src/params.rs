//! Client-only runtime parameters and argv parsing.

use clap::{parser::ValuesRef, ArgMatches};
use std::{fmt, path::PathBuf, sync::Arc};

use log::{debug, error};
use sinkd_core::{
    config::{self},
    outcome::Outcome,
    parameters::{create_log_dir, get_log_path, DaemonType, SharedDaemonParams},
};

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

impl ClientParameters {
    pub fn from_matches(matches: &ArgMatches) -> Outcome<Self> {
        let debug = matches.get_count("debug");
        create_log_dir(debug)?;

        let (daemon_type, system_config, user_configs, client_state_dir_override) =
            match matches.subcommand() {
                Some(("init", init_m)) => {
                    let client_state_dir_override = init_m
                        .get_one::<String>("client-state-dir")
                        .map(|s| PathBuf::from(s.trim()))
                        .filter(|p| !p.as_os_str().is_empty());
                    (
                        DaemonType::UnixClient,
                        init_m.get_one("system-config"),
                        init_m.get_many("user-configs"),
                        client_state_dir_override,
                    )
                }
                Some(_) => {
                    let windows = matches.get_flag("windows-daemon");
                    let daemon_type = if windows {
                        DaemonType::WindowsClient
                    } else {
                        DaemonType::UnixClient
                    };
                    let client_state_dir_override = matches
                        .get_one::<String>("client-state-dir")
                        .map(|s| PathBuf::from(s.trim()))
                        .filter(|p| !p.as_os_str().is_empty());
                    (
                        daemon_type,
                        matches.get_one("system-config"),
                        matches.get_many("user-configs"),
                        client_state_dir_override,
                    )
                }
                None => return sinkd_core::bad!("expected subcommand"),
            };

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

        let params = Self {
            shared,
            system_config: resolve_system_config(system_config)?,
            user_configs: resolve_user_configs(user_configs)?,
            client_state_dir_override,
        };

        if params.shared.debug > 0 {
            println!("{params}");
        }

        Ok(params)
    }
}

fn resolve_system_config(system_config: Option<&String>) -> Outcome<Arc<PathBuf>> {
    let cfg_path: PathBuf;
    if let Some(sys_cfg) = system_config {
        debug!("resolve_system_config>> passed in: {sys_cfg}");
        match config::resolve(sys_cfg) {
            Ok(normalized) => {
                if normalized.is_dir() {
                    return sinkd_core::bad!(
                        "{} is a directory not a file, aborting",
                        normalized.display()
                    );
                } else if normalized.exists() {
                    cfg_path = normalized;
                } else {
                    return sinkd_core::bad!("{} does not exist", normalized.display());
                }
            }
            Err(e) => return sinkd_core::bad!("system config path error: {}", e),
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
                return sinkd_core::bad!(
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

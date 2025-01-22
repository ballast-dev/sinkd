use clap::{parser::ValuesRef, ArgMatches};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{config, fancy, outcome::Outcome};

#[derive(PartialEq, Clone, Debug)]
pub enum DaemonType {
    UnixClient,
    UnixServer,
    WindowsClient,
    WindowsServer,
}

// TODO: move this into section of /etc/sinkd.conf
pub struct Parameters {
    pub daemon_type: DaemonType,
    pub verbosity: u8,
    pub debug: u8,
    pub log_path: PathBuf,
    pub system_config: Arc<PathBuf>,
    pub user_configs: Arc<Vec<PathBuf>>,
}

impl std::fmt::Display for Parameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&fancy::format(
            &format!(
                r#"🎨 Parameters 🔍
daemon_type:{:?}
verbosity:{}
debug:{}
log_path:{}
system_config:{}
user configs: [{}]
"#,
                self.daemon_type,
                self.verbosity,
                self.debug,
                self.log_path.display(),
                self.system_config.display(),
                self.user_configs
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            fancy::Attrs::Bold,
            fancy::Colors::Yellow,
        ))
    }
}

impl Parameters {
    pub fn new() -> Self {
        Parameters {
            daemon_type: DaemonType::UnixClient,
            verbosity: 0,
            debug: 0,
            log_path: PathBuf::new(),
            system_config: Arc::new(PathBuf::new()),
            user_configs: Arc::new(Vec::new()),
        }
    }

    pub fn from(matches: &ArgMatches) -> Outcome<Self> {
        let (system_config, user_configs, daemon_type) = match matches.subcommand() {
            Some(("client", submatches)) => {
                let system_config = submatches.get_one("system-config");
                let user_configs = submatches.get_many("user-configs");
                let daemon_type = if matches.get_flag("windows-daemon") {
                    DaemonType::WindowsClient
                } else {
                    DaemonType::UnixClient
                };
                (system_config, user_configs, daemon_type)
            }
            _ => {
                let daemon_type = if matches.get_flag("windows-daemon") {
                    DaemonType::WindowsServer
                } else {
                    DaemonType::UnixServer
                };
                (None, None, daemon_type)
            }
        };

        let debug = matches.get_count("debug");
        Self::create_log_dir(debug);

        let params = Parameters {
            daemon_type: daemon_type.clone(),
            verbosity: match (debug, matches.get_count("verbose")) {
                (d, _) if d > 0 => 4, // if debugging -> full verbosity
                (_, 0) => 2,          // default to warn log level  TODO: make this obsolete
                (_, v) => v,
            },
            debug: match debug {
                1 | 2 => debug,
                d if d > 2 => {
                    println!("debug only has two levels");
                    2
                }
                _ => 0,
            },
            log_path: Self::get_log_path(debug, &daemon_type),
            system_config: match daemon_type {
                DaemonType::UnixClient => Self::resolve_system_config(system_config)?,
                DaemonType::UnixServer => Arc::new(PathBuf::new()),
                DaemonType::WindowsClient => Self::resolve_system_config(system_config)?,
                DaemonType::WindowsServer => Arc::new(PathBuf::new()),
            },
            user_configs: match daemon_type {
                DaemonType::UnixClient => Self::resolve_user_configs(user_configs)?,
                DaemonType::UnixServer => Arc::new(vec![]),
                DaemonType::WindowsClient => Self::resolve_user_configs(user_configs)?,
                DaemonType::WindowsServer => Arc::new(vec![]),
            },
        };

        if params.debug > 0 {
            println!("{}", &params);
        }

        Ok(params)
    }

    fn create_log_dir(debug: u8) -> Outcome<()> {
        let path = if debug >= 1 {
            Path::new("/tmp/sinkd")
        } else {
            Path::new("/var/log/sinkd")
        };

        if !path.exists() {
            if debug == 0 && !config::have_permissions() {
                return bad!("Need elevated permissions to create {}", path.display());
            }
            match fs::create_dir_all(path) {
                Ok(()) => Ok(()),
                Err(e) => bad!("Unable to create '{}'  {}", path.display(), e),
            }
        } else {
            Ok(())
        }
    }

    fn get_log_path(debug: u8, daemon_type: &DaemonType) -> PathBuf {
        let base_dir = if debug > 0 {
            "/tmp/sinkd"
        } else {
            "/var/log/sinkd"
        };

        match daemon_type {
            DaemonType::UnixClient => PathBuf::from(format!("{}/client.log", base_dir)),
            DaemonType::UnixServer => PathBuf::from(format!("{}/server.log", base_dir)),
            DaemonType::WindowsClient => PathBuf::from(format!("{}/client.log", base_dir)),
            DaemonType::WindowsServer => PathBuf::from(format!("{}/server.log", base_dir)),
        }
    }

    //?  -- server config --
    //?  this will be outside of client config
    //?  /srv/sinkd/sinkd_server.conf
    //?  /opt/sinkd/sinkd_server.conf
    //?  -> the server files will reside in
    //?  /opt/sinkd/srv/...

    // If command line argument given, that supercedes precedence
    // else default path will be read
    fn resolve_system_config(system_config: Option<&String>) -> Outcome<Arc<PathBuf>> {
        // FIXME: need to setup "server_config" which is separate from system/user
        let cfg_path: PathBuf;
        if let Some(sys_cfg) = system_config {
            debug!("resolve_system_config>> passed in: {}", sys_cfg);
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

    // If command line argument supplied, system config not read
    // list of users are supplied from system config
    pub fn resolve_user_configs(
        user_configs: Option<ValuesRef<String>>,
    ) -> Outcome<Arc<Vec<PathBuf>>> {
        let mut resolved_configs = Vec::<PathBuf>::new();

        // safe unwrap due to default args
        if let Some(usr_cfgs) = user_configs {
            for cfg in usr_cfgs {
                let normalized = config::resolve(&cfg.to_string())?;
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

            // WARN: user configs are pulled from system and additionally supplied
            // through command line arg
            for _cfg in default_cfgs {
                match config::resolve(_cfg) {
                    Ok(resolved_user_config) => resolved_configs.push(resolved_user_config),
                    Err(e) => error!("Unable to resolve {_cfg}  {e}"),
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
}

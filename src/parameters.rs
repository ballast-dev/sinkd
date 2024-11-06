use clap::parser::ValuesRef;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{config, fancy, outcome::Outcome};

#[derive(PartialEq, Clone)]
pub enum DaemonType {
    Client,
    Server,
}

// TODO: move this into section of /etc/sinkd.conf
pub struct Parameters {
    pub daemon_type: DaemonType,
    pub verbosity: u8,
    pub clear_logs: bool,
    pub debug: bool,
    pub log_path: PathBuf,
    pub pid_path: PathBuf,
    pub system_config: Arc<PathBuf>,
    pub user_configs: Arc<Vec<PathBuf>>,
}

impl std::fmt::Display for Parameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&fancy::format(
            &format!(
                r#"🎨 Parameters 🔍
daemon_type:{}
verbosity:{}
clear_logs:{}
debug:{}
log_path:{}
pid_path:{}
system_config:{}
user configs: [{}]
"#,
                if self.daemon_type == DaemonType::Client {
                    "client"
                } else {
                    "server"
                },
                self.verbosity,
                self.clear_logs,
                self.debug,
                self.log_path.display(),
                self.pid_path.display(),
                self.system_config.display(),
                self.user_configs
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            fancy::Attrs::Bold,
            fancy::Colors::Yellow,
        ))
    }
}

impl Parameters {
    pub fn new(
        daemon_type: DaemonType,
        verbosity: u8,
        debug: bool,
        system_config: Option<&String>,
        user_configs: Option<ValuesRef<String>>,
    ) -> Outcome<Self> {
        Parameters::create_log_dir(debug)?;
        Ok(Parameters {
            daemon_type: daemon_type.clone(),
            verbosity: match (debug, verbosity) {
                (true, _) => 4,
                (false, 0) => 2, // default to warn log level
                (_, v) => v,
            },
            clear_logs: debug,
            debug,
            log_path: Self::get_log_path(debug, &daemon_type),
            pid_path: Self::get_pid_path(debug, &daemon_type),
            system_config: match daemon_type {
                DaemonType::Client => Self::resolve_system_config(system_config)?,
                DaemonType::Server => Arc::new(PathBuf::new()),
            },
            user_configs: match daemon_type {
                DaemonType::Client => Self::resolve_user_configs(user_configs)?,
                DaemonType::Server => Arc::new(vec![]),
            },
        })
    }

    fn create_log_dir(debug: bool) -> Outcome<()> {
        let path = if debug {
            Path::new("/tmp/sinkd")
        } else {
            Path::new("/var/log/sinkd")
        };

        if !path.exists() {
            if !debug && !config::have_permissions() {
                return bad!("Need elevated permissions to create /var/sinkd/");
            }
            match fs::create_dir_all(path) {
                Ok(()) => Ok(()),
                Err(e) => bad!("Unable to create '{}'  {}", path.display(), e),
            }
        } else {
            Ok(())
        }
    }

    fn get_log_path(debug: bool, daemon_type: &DaemonType) -> PathBuf {
        match (debug, daemon_type) {
            (true, DaemonType::Client) => PathBuf::from("/tmp/sinkd/client.log"),
            (true, DaemonType::Server) => PathBuf::from("/tmp/sinkd/server.log"),
            (false, DaemonType::Client) => PathBuf::from("/var/log/sinkd/client.log"),
            (false, DaemonType::Server) => PathBuf::from("/var/log/sinkd/server.log"),
        }
    }

    fn get_pid_path(debug: bool, daemon_type: &DaemonType) -> PathBuf {
        match (debug, daemon_type) {
            (true, DaemonType::Client) => PathBuf::from("/tmp/sinkd/client.pid"),
            (true, DaemonType::Server) => PathBuf::from("/tmp/sinkd/server.pid"),
            (false, DaemonType::Client) => PathBuf::from("/var/log/sinkd/client.pid"),
            (false, DaemonType::Server) => PathBuf::from("/var/log/sinkd/server.pid"),
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

        if system_config.is_some() {
            println!("DEBUG>> {}", system_config.unwrap());
            match config::resolve(system_config.unwrap()) {
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

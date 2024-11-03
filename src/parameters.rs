use clap::parser::ValuesRef;
// Common Utilities
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{fancy, outcome::Outcome, utils};

#[derive(PartialEq)]
pub enum DaemonType {
    Client,
    Server,
}

// TODO: move this into section of /etc/sinkd.conf
pub struct Parameters<'a> {
    pub daemon_type: &'a DaemonType,
    pub verbosity: u8,
    pub clear_logs: bool,
    pub debug: bool,
    pub log_path: Arc<&'a Path>,
    pub pid_path: Arc<&'a Path>,
    pub system_config: Arc<PathBuf>,
    pub user_configs: Arc<Vec<PathBuf>>,
}

impl<'a> std::fmt::Display for Parameters<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self.daemon_type == DaemonType::Client {
            f.write_str("daemon:type:client")
        } else {
            f.write_str("daemon:type:server")
        }
    }
}

impl<'a> Parameters<'a> {
    pub fn new(
        daemon_type: &'a DaemonType,
        verbosity: u8,
        debug: bool,
        system_config: Option<&String>,
        user_configs: Option<ValuesRef<String>>,
    ) -> Outcome<Self> {
        Parameters::create_log_dir(debug)?;
        Ok(Parameters {
            daemon_type,
            verbosity: match (debug, verbosity) {
                (true, _) => 4,
                (false, 0) => 2, // default to warn log level
                (_, v) => v,
            },
            clear_logs: if debug { true } else { false },
            debug,
            log_path: Self::get_log_path(debug, daemon_type),
            pid_path: Self::get_pid_path(debug, daemon_type),
            system_config: Self::resolve_system_config(daemon_type, system_config)?,
            user_configs: Self::resolve_user_configs(daemon_type, user_configs)?,
        })
    }

    fn create_log_dir(debug: bool) -> Outcome<()> {
        let path = if debug {
            Path::new("/tmp/sinkd")
        } else {
            Path::new("/var/log/sinkd")
        };

        if !path.exists() {
            if !debug && !utils::have_permissions() {
                return bad!("Need elevated permissions to create /var/sinkd/");
            }
            match fs::create_dir_all(path) {
                Ok(_) => Ok(()),
                Err(e) => bad!("Unable to create '{}'  {}", path.display(), e),
            }
        } else {
            Ok(())
        }
    }

    fn get_log_path(debug: bool, daemon_type: &'a DaemonType) -> Arc<&Path> {
        match (debug, daemon_type) {
            (true, DaemonType::Client) => Arc::new(Path::new("/tmp/sinkd/client.log")),
            (true, DaemonType::Server) => Arc::new(Path::new("/tmp/sinkd/server.log")),
            (false, DaemonType::Client) => Arc::new(Path::new("/var/log/sinkd/client.log")),
            (false, DaemonType::Server) => Arc::new(Path::new("/var/log/sinkd/server.log")),
        }
    }

    fn get_pid_path(debug: bool, daemon_type: &'a DaemonType) -> Arc<&Path> {
        match (debug, daemon_type) {
            (true, DaemonType::Client) => Arc::new(Path::new("/tmp/sinkd/client.pid")),
            (true, DaemonType::Server) => Arc::new(Path::new("/tmp/sinkd/server.pid")),
            (false, DaemonType::Client) => Arc::new(Path::new("/var/log/sinkd/client.pid")),
            (false, DaemonType::Server) => Arc::new(Path::new("/var/log/sinkd/server.pid")),
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
    fn resolve_system_config(
        daemon_type: &'a DaemonType,
        system_config: Option<&String>,
    ) -> Outcome<Arc<PathBuf>> {
        // FIXME: need to setup "server_config" which is separate from system/user
        if *daemon_type == DaemonType::Server {
            return Ok(Arc::new(PathBuf::from("not-used")));
        }

        let cfg_path: PathBuf;

        if system_config.is_some() {
            println!("DEBUG>> {}", system_config.unwrap());
            match utils::resolve(system_config.unwrap()) {
                Ok(normalized) => {
                    if normalized.is_dir() {
                        return bad!(
                            "{} is a directory not a file, aborting",
                            normalized.display()
                        );
                    } else if normalized.exists() {
                        cfg_path = PathBuf::from(normalized);
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

        Ok(Arc::new(cfg_path))
    }

    // If command line argument supplied, system config not read
    // list of users are supplied from system config
    pub fn resolve_user_configs(
        daemon_type: &'a DaemonType,
        user_configs: Option<ValuesRef<String>>,
    ) -> Outcome<Arc<Vec<PathBuf>>> {
        if *daemon_type == DaemonType::Server {
            return Ok(Arc::new(vec![PathBuf::from("not-used")]));
        }

        let mut resolved_configs = Vec::<PathBuf>::new();

        //let mut resolved_configs = vec![
        //    resolve("~/.config/sinkd/sinkd.conf")?,
        //    resolve("~/sinkd.conf")?,
        //];

        // safe unwrap due to default args
        if let Some(usr_cfgs) = user_configs {
            for cfg in usr_cfgs {
                let normalized = utils::resolve(&cfg.to_string())?;
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
                //match utils::resolve(_cfg) {
                //    Ok(usr_cfg) =>
                //}
            }
        }
        Ok(Arc::new(resolved_configs))
    }
}

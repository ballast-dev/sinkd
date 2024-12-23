use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{bad, outcome::Outcome, parameters::Parameters};

// these are serially parsable
#[derive(Debug, Serialize, Deserialize)]
struct Anchor {
    path: PathBuf,
    interval: Option<u64>,
    excludes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SysConfig {
    server_addr: String,
    users: Vec<String>,
    anchors: Option<Vec<Anchor>>,
}

impl SysConfig {
    fn new() -> SysConfig {
        SysConfig {
            server_addr: String::new(),
            users: Vec::new(),
            anchors: Some(Vec::new()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct UserConfig {
    anchors: Vec<Anchor>,
}

#[allow(dead_code)]
#[derive(PartialEq)]
enum ParseError {
    FileNotFound,
    InvalidSyntax(String),
    NoUserFound,
}

// impl std::fmt::Display for ParseError {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         match &*self {
//             ParseError::FileNotFound => write!(f, "file not found"),
//             ParseError::InvalidSyntax(err) => write!(f, "invalid syntax {}", err),
//             ParseError::ReadOnly => write!(f, "readonly"),
//             ParseError::NoUserFound => write!(f, "no user found"),
//         }
//     }
// }

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

    fn parse_configs(&mut self, params: &Parameters) -> Outcome<()> {
        if let Err(e) = self.parse_sys_config(&params.system_config) {
            match e {
                ParseError::InvalidSyntax(syn) => {
                    return bad!(
                        "Invalid sytax in '{}': {}",
                        &params.system_config.display(),
                        syn
                    );
                }
                ParseError::FileNotFound => {
                    return bad!("File not found: '{}'", &params.system_config.display());
                }
                ParseError::NoUserFound => return bad!("No user found"),
            }
        }

        // TODO: create a "sinkd group" in /etc/sinkd.conf
        // TODO: to store the server files in, i.e. /srv/sinkd/<group_name>/<abs_path>
        if let Err(ParseError::NoUserFound) = self.parse_user_configs(&params.user_configs) {
            warn!("No user was loaded into sinkd, using only system configs");
        }
        Ok(())
    }

    fn parse_sys_config(&mut self, sys_config: &PathBuf) -> Result<(), ParseError> {
        match fs::read_to_string(sys_config) {
            Err(_) => Err(ParseError::FileNotFound),
            Ok(output) => match toml::from_str(&output) {
                Err(error) => Err(ParseError::InvalidSyntax(error.to_string())),
                Ok(toml_parsed) => {
                    self.sys = toml_parsed; // NOTE: converted into Rust via serde lib
                    Ok(())
                }
            },
        }
    }

    fn parse_user_configs(&mut self, user_configs: &Vec<PathBuf>) -> Result<(), ParseError> {
        for user_config in user_configs {
            match ConfigParser::get_user_config(user_config) {
                Ok(_usr_cfg) => {
                    let _ = &self.users.insert(user_config.clone(), _usr_cfg);
                    continue;
                }
                Err(error) => match error {
                    ParseError::FileNotFound => {
                        error!("File not found: {}", user_config.display());
                    }
                    ParseError::InvalidSyntax(syntax) => {
                        error!("Invalid syntax in: {}: {}", user_config.display(), syntax);
                    }
                    _ => (),
                },
            }
        }

        if self.users.is_empty() {
            return Err(ParseError::NoUserFound);
        }
        Ok(())
    }

    fn get_user_config(user_config: &PathBuf) -> Result<UserConfig, ParseError> {
        match fs::read_to_string(user_config) {
            Err(_) => Err(ParseError::FileNotFound),
            Ok(output) => match toml::from_str(&output) {
                Err(error) => Err(ParseError::InvalidSyntax(error.to_string())),
                Ok(toml_parsed) => {
                    let user_config: UserConfig = toml_parsed;
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
}

pub type InodeMap = HashMap<PathBuf, Inode>;

pub fn get(params: &Parameters) -> Outcome<(String, InodeMap)> {
    let mut parser = ConfigParser::new();
    parser.parse_configs(params)?;

    let mut inode_map: InodeMap = HashMap::new();

    for cfg in parser.users.values() {
        for anchor in &cfg.anchors {
            // let excludes = anchor.excludes.is_some().or()
            inode_map.entry(anchor.path.clone()).or_insert(Inode {
                excludes: anchor.excludes.clone().unwrap_or(vec![]),
                interval: Duration::from_secs(anchor.interval.unwrap_or(5)),
                last_event: Instant::now(),
                event: false,
            });
        }
    }

    if let Some(anchors) = &parser.sys.anchors {
        for anchor in anchors {
            inode_map.entry(anchor.path.clone()).or_insert(Inode {
                excludes: anchor.excludes.clone().unwrap_or(vec![]),
                interval: Duration::from_secs(anchor.interval.unwrap_or(5)),
                last_event: Instant::now(),
                event: false,
            });
        }
    }
    Ok((parser.sys.server_addr, inode_map))
}

pub fn have_permissions() -> bool {
    unsafe {
        // get effective user id
        libc::geteuid() == 0
    }
}

pub fn get_hostname() -> Outcome<String> {
    let name = hostname::get()?;
    Ok(name.to_string_lossy().into_owned())
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
        p.push(path.strip_prefix("~/").unwrap());
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

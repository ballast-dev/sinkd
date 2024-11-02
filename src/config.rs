use serde::{Deserialize, Serialize};
// Serialized structures from Configuration
use clap::parser::ValuesRef;
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::outcome::Outcome;
use crate::utils::{self, Parameters};

// these are serially parsable
#[derive(Debug, Serialize, Deserialize)]
struct Anchor {
    path: PathBuf,
    interval: u64,
    excludes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SysConfig {
    server_addr: String,
    users: Vec<String>,
    shares: Vec<Anchor>,
}

impl SysConfig {
    fn new() -> SysConfig {
        SysConfig {
            server_addr: String::new(),
            users: Vec::new(),
            shares: Vec::new(),
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

    // If just system configs are used that is enough.
    // The storage of files will be on a group name basis.
    // The name shall be config driven? Maybe a temp file to
    // store the hash of this sinkd group...
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
                _ => {
                    return bad!("load_configs unknown condition");
                }
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
                    //? toml_parsed is converted into Rust via serde lib
                    self.sys = toml_parsed;
                    Ok(())
                }
            },
        }
    }

    fn parse_user_configs(&mut self, user_configs: &Vec<PathBuf>) -> Result<(), ParseError> {
        let mut _user_parsed = false;
        if user_configs.is_empty() {
            // default behavior is to check system for users
            for user in &self.sys.users {
                let user_config =
                    PathBuf::from_str(&format!("/home/{}/.config/sinkd.conf", user)).unwrap();
                match ConfigParser::get_user_config(&user_config) {
                    Ok(_usr_cfg) => {
                        let _ = &self.users.insert(user_config, _usr_cfg);
                        _user_parsed = true;
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
        } else {
            for user_config in user_configs {
                match ConfigParser::get_user_config(user_config) {
                    Ok(_usr_cfg) => {
                        let _ = &self.users.insert(user_config.clone(), _usr_cfg);
                        _user_parsed = true;
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
        }

        if !_user_parsed {
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

    for anchor in parser.sys.shares.iter() {
        // if !inode_map.contains_key(&anchor.path) {
        inode_map.entry(anchor.path.clone()).or_insert(Inode {
            excludes: anchor.excludes.clone(),
            interval: Duration::from_secs(anchor.interval),
            last_event: Instant::now(),
            event: false,
        });
    }
    for (_, cfg) in parser.users.iter() {
        for anchor in &cfg.anchors {
            inode_map.entry(anchor.path.clone()).or_insert(Inode {
                excludes: anchor.excludes.clone(),
                interval: Duration::from_secs(anchor.interval),
                last_event: Instant::now(),
                event: false,
            });
        }
    }
    Ok((parser.sys.server_addr, inode_map))
}

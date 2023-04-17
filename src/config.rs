use serde::{Deserialize, Serialize};
// Serialized structures from Configuration
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

// these are serially parsable
#[derive(Debug, Serialize, Deserialize)]
struct Anchor {
    path: PathBuf,
    interval: u64,
    excludes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SysParser {
    server_addr: String,
    users: Vec<String>,
    shares: Vec<Anchor>,
}

impl SysParser {
    fn new() -> SysParser {
        SysParser {
            server_addr: String::new(),
            users: Vec::new(),
            shares: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct UserParser {
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
    sys: SysParser,
    users: HashMap<String, UserParser>,
}

#[doc = "don't need a class to operate"]
impl ConfigParser {
    // system level config that will house user based configs
    fn new() -> ConfigParser {
        ConfigParser {
            sys: SysParser::new(),
            users: HashMap::new(),
        }
    }

    /// If just system configs are used that is enough.
    /// The storage of files will be on a group name basis.
    /// The name shall be config driven? Maybe a temp file to
    /// store the hash of this sinkd group...
    fn load_configs(&mut self) -> Result<(), String> {
        if let Err(e) = self.load_sys_config() {
            match e {
                ParseError::InvalidSyntax(syn) => {
                    return Err(format!("Invalid sytax in '/etc/sinkd.conf': {}", syn));
                }
                ParseError::FileNotFound => {
                    return Err(format!("File not found: '/etc/sinkd.conf'"));
                }
                _ => {
                    return Err(format!("sysconfig unknown condition"));
                }
            }
        }

        // TODO: create a "sinkd group" in /etc/sinkd.conf
        // TODO: to store the server files in, i.e. /srv/sinkd/<group_name>/<abs_path>
        if let Err(ParseError::NoUserFound) = self.load_user_configs() {
            warn!("No user was loaded into sinkd, using only system configs");
        }
        Ok(())
    }

    fn load_sys_config(&mut self) -> Result<(), ParseError> {
        match fs::read_to_string("/etc/sinkd.conf") {
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

    fn load_user_configs(&mut self) -> Result<(), ParseError> {
        let mut _user_loaded = false;
        for user in &self.sys.users {
            let user_config = format!("/home/{}/.config/sinkd.conf", user);
            match ConfigParser::get_user_config(&user_config) {
                Ok(_usr_cfg) => {
                    let _ = &self.users.insert(user.clone(), _usr_cfg);
                    _user_loaded = true;
                    continue;
                }
                Err(error) => match error {
                    ParseError::FileNotFound => {
                        error!("File not found: {}", user_config);
                    }
                    ParseError::InvalidSyntax(syntax) => {
                        error!("Invalid syntax in: {}: {}", user_config, syntax);
                    }
                    _ => (),
                },
            }
        }
        if !_user_loaded {
            return Err(ParseError::NoUserFound);
        }
        Ok(())
    }

    fn get_user_config(user_config: &str) -> Result<UserParser, ParseError> {
        match fs::read_to_string(&user_config) {
            Err(_) => Err(ParseError::FileNotFound),
            Ok(output) => match toml::from_str(&output) {
                Err(error) => Err(ParseError::InvalidSyntax(error.to_string())),
                Ok(toml_parsed) => {
                    let user_config: UserParser = toml_parsed;
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

pub fn get() -> Result<(String, InodeMap), String> {
    let mut parser = ConfigParser::new();
    parser.load_configs()?;

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

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
    ReadOnly,
    NoUserFound,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            ParseError::FileNotFound => write!(f, "file not found"),
            ParseError::InvalidSyntax(err) => write!(f, "invalid syntax {}", err),
            ParseError::ReadOnly => write!(f, "readonly"),
            ParseError::NoUserFound => write!(f, "no user found"),
        }
    }
}

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

    //? packager will take care of loading files
    fn load_configs(&mut self) -> bool {
        if let Err(e) = self.load_sys_config() {
            match e {
                ParseError::InvalidSyntax(syn) => error!("{}", syn),
                _ => error!("{}", e),
            }
            return false;
        }

        if let Err(e) = self.load_user_configs() {
            error!("{}", e);
            return false;
        }

        true
    }

    fn load_sys_config(&mut self) -> Result<(), ParseError> {
        match fs::read_to_string("/etc/sinkd.conf") {
            Err(error) => {
                error!("unable to open /etc/sinkd.conf, {}", error);
                return Err(ParseError::FileNotFound);
            }
            Ok(output) => match toml::from_str(&output) {
                Err(error) => {
                    // error!("couldn't parse '/etc/sinkd.conf' {}", error);
                    let invalid_syntax = ParseError::InvalidSyntax(error.to_string());
                    Err(invalid_syntax)
                }
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
            match ConfigParser::get_user_config(user) {
                Ok(_usr_cfg) => {
                    let _ = &self.users.insert(user.clone(), _usr_cfg);
                    _user_loaded = true;
                    continue;
                }
                Err(_) => {
                    warn!("user '{}' not found", user)
                }
            }
        }
        if !_user_loaded {
            return Err(ParseError::NoUserFound);
        }
        Ok(())
    }

    fn get_user_config(username: &str) -> Result<UserParser, ParseError> {
        // use home_dir which should work on the *nixes
        // if let Some(home) = env::home_dir() {
        //     use crate::utils::{Attrs::*, Colors::*};
        //     print_fancyln(format!("HOME{} ==>> print off environment", home.display()).as_str(), BOLD, GREEN);
        //     for (key, value) in env::vars_os() {
        //         println!("{:?}: {:?}", key, value);
        //     }
        // }
        let config_file = format!("/home/{}/.config/sinkd.conf", &username);

        match fs::read_to_string(&config_file) {
            Err(error) => {
                error!("unable to open {}, {}", &config_file, error);
                return Err(ParseError::FileNotFound);
            }
            Ok(output) => match toml::from_str(&output) {
                Err(error) => {
                    let err_str = format!("couldn't parse '{}' {}", &config_file, error);
                    let invalid_syntax = ParseError::InvalidSyntax(err_str);
                    return Err(invalid_syntax);
                }
                Ok(toml_parsed) => {
                    debug!("user config parsed????");
                    //? toml_parsed is converted into Rust via serde lib
                    let user_config: UserParser = toml_parsed;
                    return Ok(user_config);
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct Inode {
    pub excludes: Vec<String>, // holds wildcards
    pub interval: Duration,
    pub last_event: Instant,
    pub event: bool,
}

pub type InodeMap = HashMap<PathBuf, Inode>;

pub fn get() -> (String, InodeMap) {
    let mut parser = ConfigParser::new();
    if !parser.load_configs() {
        error!("FATAL couldn't load configurations");
        panic!("FATAL couldn't load configurations")
    }

    let mut inode_map: InodeMap = HashMap::new();

    for anchor in parser.sys.shares.iter() {
        // if !inode_map.contains_key(&anchor.path) {
        inode_map.entry(anchor.path.clone()).or_insert(Inode {
            excludes: anchor.excludes.clone(),
            interval: Duration::from_secs(anchor.interval),
            last_event: Instant::now(),
            event: false,
        });
        // } else {
        //     error!("[sys_config] inode_map already contains path(key): {}", &anchor.path.display());
        // }
    }
    for (_, cfg) in parser.users.iter() {
        for anchor in &cfg.anchors {
            // if !inode_map.contains_key(&anchor.path) {
            inode_map.entry(anchor.path.clone()).or_insert(Inode {
                excludes: anchor.excludes.clone(),
                interval: Duration::from_secs(anchor.interval),
                last_event: Instant::now(),
                event: false,
            });
            // } else {
            //     error!("[usr_config] inode_map already contains path(key): {}", &anchor.path.display());
            // }
        }
    }

    return (parser.sys.server_addr, inode_map);
}

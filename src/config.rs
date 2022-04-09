// Serialized structures from Configuration
use std::{
    path::PathBuf,
    fs,
    collections::HashMap,
    time::{Duration, Instant}
};
use crate::utils::print_fancyln;


// these are serially parsable 
#[derive(Debug, Serialize, Deserialize)]
pub struct Anchor {
    pub path: PathBuf,
    pub interval: u64,
    pub excludes: Vec<String>,
}

// anchors are user directories to watch 
// these are defined in ~/.config/sinkd.conf
impl Anchor {
    pub fn new() -> Anchor {
        Anchor {
            path: PathBuf::from("invalid"),
            interval: 5, // defaults to 5 secs?
            excludes: Vec::new()
        }
    }
}
#[derive(Debug)]
pub struct Inode {
    pub excludes: Vec::<String>, // holds wildcards
    pub interval: Duration,
    pub last_event: Instant,
    pub event: bool 
}

pub type InodeMap = HashMap<PathBuf, Inode>; 

// to show case function overloading 
pub trait BuildAnchor {
    fn add_watch(&mut self, path: PathBuf, users: Vec<String>, interval: u32, excludes: Vec<String>);
}

pub trait FormedAnchor {
    fn add_watch(&mut self, anchor: Anchor);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SysConfig {
    pub server_addr: String,
    pub users: Vec<String>,
    pub shares: Vec<Anchor>,
}

impl SysConfig {
    fn new() -> SysConfig {
        SysConfig{
            server_addr: String::new(),
            users: Vec::new(),
            shares: Vec::new()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserConfig {
    pub anchors: Vec<Anchor>
}

#[allow(dead_code)]
#[derive(PartialEq)]
pub enum ConfigError {
    FileNotFound,
    InvalidSyntax(String),
    ReadOnly,
    NoUserFound
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            ConfigError::FileNotFound       => write!(f, "file not found"),
            ConfigError::InvalidSyntax(err) => write!(f, "invalid syntax {}", err),
            ConfigError::ReadOnly           => write!(f, "readonly"),
            ConfigError::NoUserFound        => write!(f, "no user found"),
        }
    }
}

pub struct Config {
    pub sys: SysConfig,
    pub users: HashMap<String, UserConfig>,
}


#[doc="don't need a class to operate"]
impl Config {
    // system level config that will house user based configs
    pub fn new() -> Config {
        Config {
            sys: SysConfig::new(),
            users: HashMap::new()
        }
    }

    //? packager will take care of loading files
    pub fn init(&mut self) -> bool {
        match self.load_sys_config() {
            Ok(_) => {}
            Err(e) => {
                match e {
                    ConfigError::InvalidSyntax(syn) => {
                        error!("{}", syn)
                    }, 
                    _ => { error!("{}", e) }
                }
                return false;
            }
        }

        match self.load_user_configs() {
            Ok(_) => { return true; }, // loaded both system and user
            Err(e) => {
                error!("{}", e);
                return false;
            }
        }

    }

    pub fn load_sys_config(&mut self) -> Result<(), ConfigError> {
        match Config::get_sys_config() {
            Ok(sys_config) => {
                self.sys = sys_config;
                return Ok(());
            },
            Err(e) => { Err(e) } // fire it up the chain
        }
    }

    pub fn load_user_configs(&mut self) -> Result<(), ConfigError> {
        let mut _user_loaded = false;
        for user in &self.sys.users {
            match Config::get_user_config(&user) {
                Ok(_usr_cfg) => { 
                    let _ = &self.users.insert(user.clone(), _usr_cfg); 
                    _user_loaded = true;
                    continue;
                },
                Err(_) => {
                    warn!("user '{}' not found", user)
                }
            }
        }
        if !_user_loaded {
            return Err(ConfigError::NoUserFound)
        } 
        Ok(())
    }

    // pub fn add_user_config() {}
    // pub fn rm_user_config() {}
    //? Package will install '/etc/sinkd.conf'
    pub fn get_sys_config() -> Result<SysConfig, ConfigError> {
        match fs::read_to_string("/etc/sinkd.conf") {
            Err(error) => {
                error!("unable to open /etc/sinkd.conf, {}", error);
                return Err(ConfigError::FileNotFound);
            }
            Ok(output) => match toml::from_str(&output) {
                Err(error) => {
                    // error!("couldn't parse '/etc/sinkd.conf' {}", error);
                    let invalid_syntax = ConfigError::InvalidSyntax(error.to_string());
                    return Err(invalid_syntax);
                }
                Ok(toml_parsed) => {
                    //? toml_parsed is converted into Rust via serde lib
                    let sys_config: SysConfig = toml_parsed;
                    return Ok(sys_config);
                }
            }
        }
    }

    pub fn get_user_config(username: &str) -> Result<UserConfig, ConfigError> {
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
                return Err(ConfigError::FileNotFound);
            }
            Ok(output) => match toml::from_str(&output) {
                Err(error) => {
                    let err_str = format!("couldn't parse '{}' {}", &config_file, error);
                    let invalid_syntax = ConfigError::InvalidSyntax(err_str);
                    return Err(invalid_syntax);
                }
                Ok(toml_parsed) => {
                    //? toml_parsed is converted into Rust via serde lib
                    let user_config: UserConfig = toml_parsed;
                    return Ok(user_config);
                }
            }
        }
    }

}


pub fn get_map_and_server() -> (InodeMap, String) {
    // create interval hashmap 

    let mut config = Config::new();
    if !config.init() {
        error!("FATAL couldn't initialize configurations");
        panic!("FATAL couldn't initialize configurations") 
    }

    let mut inode_map: InodeMap = HashMap::new();

    for anchor in config.sys.shares.iter() {
        if !inode_map.contains_key(&anchor.path) {
            inode_map.insert(
                anchor.path.clone(),
                Inode {
                    excludes: anchor.excludes.clone(),
                    interval: Duration::from_secs(anchor.interval),
                    last_event: Instant::now(),
                    event: false
                }
            );
        } else {
            error!("[sys_config] inode_map already contains path(key): {}", &anchor.path.display());
        }
    }
    for (_, cfg) in config.users.iter() {
        for anchor in &cfg.anchors {
            if !inode_map.contains_key(&anchor.path) {
                //TODO: better way to check before inserting
                // inode_map.entry(anchor.path).or_insert(Inode{});
                inode_map.insert(
                    anchor.path.clone(),
                    Inode {
                        excludes: anchor.excludes.clone(),
                        interval: Duration::from_secs(anchor.interval),
                        last_event: Instant::now(),
                        event: false
                    }
                );
            } else {
                error!("[usr_config] inode_map already contains path(key): {}", &anchor.path.display());
            }
        }
    }
    return (inode_map, config.sys.server_addr);
}


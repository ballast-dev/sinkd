// Serialized structures from Configuration
use std::path::PathBuf;
use std::fs;
use serde::Deserialize;
use std::env;
use std::collections::HashMap;

use crate::utils::print_fancyln;

#[derive(Debug, Serialize, Deserialize)]
pub struct Anchor {
    pub path: PathBuf,
    pub interval: u32,
    pub excludes: Vec<String>
}

// impl<'de: 'a, 'a> Deserialize<'de> for Anchor {

// }

// anchors are user directories to watch 
// these are defined in ~/.config/sinkd.conf
impl Anchor {
    pub fn new() -> Anchor {
        Anchor {
            path: PathBuf::from("invalid"),
            interval: 5, // defaults to 5 secs?
            excludes: Vec::new(),
        }
    }

    pub fn from(path: PathBuf, users: Vec<String>, interval: u32, excludes: Vec<String>) -> Anchor {
        Anchor {
            path, 
            interval,
            excludes
        }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    pub fn get_path(&self) -> &PathBuf {
        return &self.path;
    }

    pub fn set_interval(&mut self, interval: u32) {
        self.interval = interval;
    }

    pub fn get_interval(&self) -> u32 {
        return self.interval;
    }

    pub fn add_exclude(&mut self, _path: PathBuf) -> bool {
        let added: bool = true;
        if added {
            return true;
        } else {
            return false;
        }
    }
}

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
    pub name: String,
    pub anchors: Vec<Anchor>
}
pub struct Config {
    // overall Config object to hold smaller 
    pub sys: SysConfig,
    pub users: Vec<UserConfig>,
}


#[doc="don't need a class to operate"]
impl Config {
    // system level config that will house user based configs
    pub fn new() -> Config {
        Config {
            sys: SysConfig::new(),
            users: Vec::new()
        }
    }
}

pub enum ConfigError {
    FileNotFound,
    InvalidSyntax(String),
    MissingField,
    ReadOnly
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            ConfigError::FileNotFound  => write!(f, "FileNotFound"),
            ConfigError::InvalidSyntax(err) => write!(f, "InvalidSyntax {}", err),
            ConfigError::MissingField  => write!(f, "MissingField"),
            ConfigError::ReadOnly      => write!(f, "Readonly"),
        }
    }
}

//? assume package will install '/etc/sinkd.conf'
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

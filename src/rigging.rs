// Definitions for sinkd

// Serialized structures from Configuration
use std::path::PathBuf;

pub trait BuildAnchor {
    fn add_watch(&mut self, path: PathBuf, users: Vec<String>, interval: u32, excludes: Vec<String>);
}

pub trait FormedAnchor {
    fn add_watch(&mut self, anchorage: Anchorage);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub users: Vec<User>,
    pub anchorages: Vec<Anchorage>,
}

impl Config{
    pub fn new() -> Config {
        Config {
            users: User::create(),
            anchorages: vec![Anchorage::new()],
        }
    }
}


// method overloading... 
impl BuildAnchor for Config {
    fn add_watch(&mut self, path: PathBuf, users: Vec<String>, interval: u32, excludes: Vec<String>) {
        self.anchorages.push(Anchorage::from(path, users, interval, excludes));
    }
}

impl FormedAnchor for Config {
    fn add_watch(&mut self, anchorage: Anchorage) {
        self.anchorages.push(anchorage);
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub host: String,
}

impl User {
    pub fn new() -> User {
        User {
            name: String::from("new_username"),
            host: String::from("new_hostname"),
        }
    }
    pub fn create() -> Vec<User> {
        vec![User::new()]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Anchorage {
    pub path: PathBuf,
    pub users: Vec<String>,
    pub interval: u32,
    pub excludes: Vec<String>,
}

impl Anchorage {
    pub fn new() -> Anchorage {
        Anchorage {
            path: PathBuf::from("invalid"),
            users: Vec::<String>::new(),
            interval: 5, // defaults to 5 secs?
            excludes: Vec::new(),
        }
    }

    pub fn from(path: PathBuf, users: Vec<String>, interval: u32, excludes: Vec<String>) -> Anchorage {
        Anchorage {
            path, 
            users,
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

    pub fn add_user(&mut self, _user: &str) -> bool {
        // if (users.count == 0) implicity means to share the anchorage
        // moves the folder from: server_root/opt/sinkd/user/anchorage
        //                  to:   server_root/opt/sinkd/share/anchorage
        // should probably lock it down to certain group `sinkd` 
        return true;
    }

    pub fn rm_user(&mut self, _user: &str) -> bool {
        // if (users.count == 0) implicity means to un-share the anchorage
        // moves the folder from: server_root/opt/sinkd/share/anchorage
        //                  to:   server_root/opt/sinkd/user/anchorage
        return true;
    }
}

pub struct TimeStamp;
use libc;
use std::ffi::CString;
extern {
    fn strftime(s: *mut libc::c_char, max: libc::size_t, format: *const libc::c_char, tm: *mut libc::tm) -> libc::size_t;
}


use libc::c_char;
use std::ops::Deref;
use std::ffi::CStr;

extern "C" {
    fn hello() -> *const c_char;
    fn goodbye(s: *const c_char);
}

struct Greeting {
    message: *const c_char,
}

impl Drop for Greeting {
    fn drop(&mut self) {
        unsafe {
            goodbye(self.message);
        }
    }
}

impl Greeting {
    fn new() -> Greeting {
        Greeting { message: unsafe { hello() } }
    }
}

impl Deref for Greeting {
    type Target = str;

    fn deref<'a>(&'a self) -> &'a str {
        let c_str = unsafe { CStr::from_ptr(self.message) };
        c_str.to_str().unwrap()
    }
}
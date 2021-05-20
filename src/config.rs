// Serialized structures from Configuration
use std::path::PathBuf;

pub trait BuildAnchor {
    fn add_watch(&mut self, path: PathBuf, users: Vec<String>, interval: u32, excludes: Vec<String>);
}

pub trait FormedAnchor {
    fn add_watch(&mut self, anchor: Anchor);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub users: Vec<User>,
    pub anchors: Vec<Anchor>,
}

impl Config{
    pub fn new() -> Config {
        Config {
            server_addr: String::new(),
            users: User::create(),
            anchors: vec![Anchor::new()],
        }
    }
}


// method overloading... 
impl BuildAnchor for Config {
    fn add_watch(&mut self, path: PathBuf, users: Vec<String>, interval: u32, excludes: Vec<String>) {
        self.anchors.push(Anchor::from(path, users, interval, excludes));
    }
}

impl FormedAnchor for Config {
    fn add_watch(&mut self, anchor: Anchor) {
        self.anchors.push(anchor);
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
pub struct Anchor {
    pub path: PathBuf,
    pub users: Vec<String>,
    pub interval: u32,
    pub excludes: Vec<String>,
}

impl Anchor {
    pub fn new() -> Anchor {
        Anchor {
            path: PathBuf::from("invalid"),
            users: Vec::<String>::new(),
            interval: 5, // defaults to 5 secs?
            excludes: Vec::new(),
        }
    }

    pub fn from(path: PathBuf, users: Vec<String>, interval: u32, excludes: Vec<String>) -> Anchor {
        Anchor {
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
        // if (users.count == 0) implicity means to share the anchor
        // moves the folder from: server_root/opt/sinkd/user/anchor
        //                  to:   server_root/opt/sinkd/share/anchor
        // should probably lock it down to certain group `sinkd` 
        return true;
    }

    pub fn rm_user(&mut self, _user: &str) -> bool {
        // if (users.count == 0) implicity means to un-share the anchor
        // moves the folder from: server_root/opt/sinkd/share/anchor
        //                  to:   server_root/opt/sinkd/user/anchor
        return true;
    }
}
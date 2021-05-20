/// Serialized structures from Configuration
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub owner: Owner,
    pub users: Vec<User>,
    pub anchor_points: Vec<AnchorPoint>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            owner: Owner::new(),
            users: User::create(),
            anchor_points: vec![AnchorPoint::new()],
        }
    }

    pub fn add_watch(
        &mut self,
        path: PathBuf,
        users: Vec<String>,
        interval: u32,
        excludes: Vec<String>,
    ) {
        self.anchor_points.push(AnchorPoint {
            path,
            users,
            interval,
            excludes,
        });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    pub key: String,
}

impl Owner {
    pub fn new() -> Owner {
        Owner {
            name: String::from("new_owner"),
            key: String::from("owner_key"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub address: String,
    pub ssh_key: String,
}

impl User {
    pub fn new() -> User {
        User {
            name: String::from("new_user"),
            address: String::from("user_addr"),
            ssh_key: String::from("user_key"),
        }
    }
    pub fn create() -> Vec<User> {
        vec![User::new()]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnchorPoint {
    pub path: PathBuf,
    pub users: Vec<String>,
    pub interval: u32,
    pub excludes: Vec<String>,
}

impl AnchorPoint {
    pub fn new() -> AnchorPoint {
        AnchorPoint {
            path: PathBuf::from("invalid"),
            users: Vec::<String>::new(),
            interval: 5, // defaults to 5 secs?
            excludes: Vec::new(),
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
        return true;
    }

    pub fn rm_user(&mut self, _user: &str) -> bool {
        return true;
    }
}

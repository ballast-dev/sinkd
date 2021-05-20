/// Serialized structures from Configuration

#[derive(Debug, Serialize, Deserialize)]
pub struct Overlook {
    pub owner: Owner,
    pub users: Vec<User>,
    pub watches: Vec<Directory>,
}

impl Overlook {
    pub fn new() -> Overlook {
        Overlook {
            owner: Owner::new(),
            users: User::create(),
            watches: Directory::create(),
        }
    }

    pub fn add_watch(&mut self, path: String, users: Vec<String>, interval: u32, excludes: Vec<String>) {
        let mut new_dir = Directory {
                path,
                users,
                interval,
                excludes,
            };
        self.watches.push(new_dir);
        
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
pub struct Directory {
    pub path: String,
    pub users: Vec<String>,
    pub interval: u32,
    pub excludes: Vec<String>,
}

impl Directory {
    pub fn new() -> Directory {
        Directory {
            path: String::from("invalid"),
            users: Vec::<String>::new(),
            interval: 5, // defaults to 5 secs? 
            excludes: Vec::new(),
        }
    }

    pub fn create() -> Vec<Directory> {
        vec![Directory::new()]
    }

}
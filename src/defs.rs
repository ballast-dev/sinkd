/// Definitions for Daemon

#[derive(Debug, Deserialize)]
pub struct Config {
    pub owner: Owner,
    pub users: Vec<User>,
    pub watches: Vec<Directory>,
}

#[derive(Debug, Deserialize)]
pub struct Owner {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub address: String,
    pub ssh_key: String,
}


#[derive(Debug, Deserialize)]
pub struct Directory {
    pub path: String,
    pub users: Vec<String>,
    pub interval: u32,
    pub excludes: Vec<String>,
}
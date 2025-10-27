use log::info;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[allow(dead_code)]
pub struct Environment {
    pub repo_root: PathBuf,
    pub client_config: PathBuf,
    pub server_config: PathBuf,
    pub client_path: PathBuf,
    pub server_path: PathBuf,
}

impl Environment {
    pub fn setup() -> Self {
        let repo_root = get_repo_root();
        let client_path = repo_root.join("test").join("client");
        let server_path = repo_root.join("test").join("server");
        if client_path.exists() {
            fs::remove_dir_all(&client_path).expect("unable to remove client_path");
        }
        if server_path.exists() {
            fs::remove_dir_all(&server_path).expect("unable to remove server_path");
        }
        fs::create_dir_all(&client_path).expect("Failed to create client directory");
        fs::create_dir_all(&server_path).expect("Failed to create server directory");

        Self {
            repo_root: repo_root.clone(),
            client_config: repo_root.join("scenario").join("sinkd.conf"),
            server_config: repo_root.join("scenario").join("etc_sinkd.conf"),
            client_path,
            server_path,
        }
    }
}

fn get_repo_root() -> PathBuf {
    let repo_root = Path::new(file!())
        .parent()
        .expect("Failed to get parent directory of the script")
        .parent()
        .expect("Failed to get grandparent directory of the script")
        .parent()
        .expect("Failed to get great-grandparent directory of the script");
    PathBuf::from(repo_root)
}

/// Removes all subdirectories within the specified directory.
pub fn remove_subfiles(directory: &Path) {
    info!("Removing subfiles in {}", directory.display());
    if directory.exists() {
        for entry in fs::read_dir(directory).expect("Failed to read directory") {
            let entry = entry.expect("Failed to get directory entry");
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)
                    .unwrap_or_else(|_| panic!("Failed to remove directory {}", path.display()));
                info!("Removed {}", path.display());
            }
        }
    }
}

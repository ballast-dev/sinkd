pub fn get_sinkd_path() -> std::path::PathBuf {
    let user = env!("USER");
    let sinkd_path = if cfg!(target_os = "macos") {
        std::path::Path::new("/Users").join(user).join(".sinkd")
    } else {
        std::path::Path::new("/home").join(user).join(".sinkd")
    };
    
    if !sinkd_path.exists() {
        match std::fs::create_dir(&sinkd_path) {
            Err(why) => println!("cannot create {:?}, {:?}", sinkd_path, why.kind()),
            Ok(_) => {},
        }
    }
    return sinkd_path;
} 
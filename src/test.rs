#[test]
fn config() {
    use crate::config as conf;
    let sys_config = match conf::get_sys_config() {
        Ok(sys_config) => { sys_config },
        Err(err) => {
            panic!("/etc/sinkd.conf ERROR {}", err);
        }
    };
    for user in sys_config.users {
        let _cfg = format!("/home/{}/.config/sinkd.conf", user);
        match std::fs::read_to_string(&_cfg) {
            Ok(_) => {
                println!("going to user {}", &user);
                match conf::get_user_config(&user) {
                    Ok(_) => {},
                    Err(e) => {
                        panic!("Error {}, {}", &_cfg, e)
                    }
                }
            },
            Err(_) => { 
                eprintln!("configuration not loaded for {}", &user) 
            }
        }
    }
}

#[test]
#[doc = "cargo test fancy -- --nocapture"]
fn fancy() {
    use crate::utils::*;
    print_fancyln("deploy the anchor matey!", Attrs::INVERSE, Colors::YELLOW); 
}

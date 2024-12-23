// #[test]
// fn config() {
//     use crate::config;

//     let (srv_addr, inode_map) = config::get();
//     println!("Server Address: {}", srv_addr);

//     //? need to somehow trinkle in verbosity in tests

//     for (path, inode) in inode_map {
//         println!("path:       {:?}", path);
//         println!("excludes:   {:?}", inode.excludes);
//         println!("interval:   {:?}", inode.interval);
//         println!("last_event: {:?}", inode.last_event);
//         println!("event:      {:?}", inode.event);
//     }
// }

// #[test]
// #[doc = "cargo test fancy -- --nocapture"]
// fn fancy() {
//     use crate::utils::*;
//     print_fancyln("deploy the anchor matey!", Attrs::INVERSE, Colors::YELLOW);
// }

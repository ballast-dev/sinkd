// Common Utilities

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

use libc::{c_char, c_uint};
use std::ffi::CString;


extern {
    fn timestamp(ret_str: *mut c_char, size: c_uint, fmt_str: *const c_char);
}


pub fn get_timestamp(fmt_str: &str) -> String {
    let ret_str = CString::new(Vec::with_capacity(25)).unwrap();
    let ret_ptr: *mut c_char = ret_str.into_raw();

    let _fmt_str = CString::new(fmt_str.as_bytes()).unwrap();
    let stamp: CString;
    unsafe { 
        timestamp(ret_ptr, 25, _fmt_str.as_ptr()); 
        stamp = CString::from_raw(ret_ptr);
    }
    let v = stamp.into_bytes();
    return String::from_utf8_lossy(&v).into_owned();
 }
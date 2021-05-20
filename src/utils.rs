// Common Utilities
use std::fmt;
use std::path;

pub fn get_sinkd_path() -> path::PathBuf {
    let user = env!("USER");
    let sinkd_path = if cfg!(target_os = "macos") {
        path::Path::new("/Users").join(user).join(".sinkd")
    } else {
        path::Path::new("/home").join(user).join(".sinkd")
    };
    
    if !sinkd_path.exists() {
        match std::fs::create_dir(&sinkd_path) {
            Err(why) => println!("cannot create {:?}, {:?}", sinkd_path, why.kind()),
            Ok(_) => {},
        }
    }
    return sinkd_path;
} 

//--------------------
// C O L O R S 
//--------------------
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum Colors {
// Foreground
    BLACK           = 30,
    RED             = 31,
    GREEN           = 32,
    YELLOW          = 33,
    BLUE            = 34,
    PURPLE          = 35,
    CYAN            = 36,
    WHITE           = 37,
    BRIGHT_BLUE     = 94,
    BRIGHT_PURPLE   = 95,
// Background
    BgBLACK         = 40,
    BgRED           = 41,
    BgGREEN         = 42,
    BgYELLOW        = 43,
    BgBLUE          = 44,
    BgPURPLE        = 45,
    BgCYAN          = 46,
    BgWHITE         = 47,
}

impl fmt::Display for Colors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match *self {
            Colors::BLACK          => write!(f, "30"),
            Colors::RED            => write!(f, "31"),
            Colors::GREEN          => write!(f, "32"),
            Colors::YELLOW         => write!(f, "33"),
            Colors::BLUE           => write!(f, "34"),
            Colors::PURPLE         => write!(f, "35"),
            Colors::CYAN           => write!(f, "36"),
            Colors::WHITE          => write!(f, "37"),
            Colors::BRIGHT_BLUE    => write!(f, "94"),
            Colors::BRIGHT_PURPLE  => write!(f, "95"),
            Colors::BgBLACK        => write!(f, "40"),
            Colors::BgRED          => write!(f, "41"),
            Colors::BgGREEN        => write!(f, "42"),
            Colors::BgYELLOW       => write!(f, "43"),
            Colors::BgBLUE         => write!(f, "44"),
            Colors::BgPURPLE       => write!(f, "45"),
            Colors::BgCYAN         => write!(f, "46"),
            Colors::BgWHITE        => write!(f, "47"),
      }
    }
}

#[allow(dead_code)]
pub enum Attrs {
// # Attributes
    NORMAL          = 0,
    BOLD            = 1,
    UNDERLINE       = 4,
    INVERSE         = 7, // foreground becomes background (vice-versa)
}

impl fmt::Display for Attrs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
       match *self {
            Attrs::NORMAL    => write!(f, "0"),
            Attrs::BOLD      => write!(f, "1"),
            Attrs::UNDERLINE => write!(f, "4"),
            Attrs::INVERSE   => write!(f, "7"),
       }
    }
}

pub fn print_fancy(arg: &str, attr: Attrs, color: Colors) {
    print!("{}", format!("\u{1b}[{};{}m{}\u{1b}[0m", attr, color, arg));
}

pub fn print_fancyln(arg: &str, attr: Attrs, color: Colors) {
    println!("{}", format!("\u{1b}[{};{}m{}\u{1b}[0m", attr, color, arg));
}

pub fn format_fancy(arg: &str, attr: Attrs, color: Colors) -> String {
    format!("\u{1b}[{};{}m{}\u{1b}[0m", attr, color, arg)
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
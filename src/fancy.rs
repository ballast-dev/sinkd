use std::fmt;

//--------------------
// C O L O R S
//--------------------
#[repr(u8)]
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum Colors {
    // Foreground
    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Purple = 35,
    Cyan = 36,
    White = 37,
    BrightBlue = 94,
    BrightPurple = 95,
    // Background
    BgBlack = 40,
    BgRed = 41,
    BgGreen = 42,
    BgYellow = 43,
    BgBlue = 44,
    BgPurple = 45,
    BgCyan = 46,
    BgWhite = 47,
}

impl fmt::Display for Colors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[repr(u8)]
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum Attrs {
    // # Attributes
    Normal = 0,
    Bold = 1,
    Underline = 4,
    Inverse = 7, // foreground becomes background (vice-versa)
}

impl fmt::Display for Attrs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[allow(dead_code)]
pub fn print(arg: &str, attr: Attrs, color: Colors) {
    print!("\u{1b}[{attr};{color}m{arg}\u{1b}[0m");
}

pub fn println(arg: &str, attr: Attrs, color: Colors) {
    println!("\u{1b}[{attr};{color}m{arg}\u{1b}[0m");
}

#[allow(dead_code)]
pub fn format(arg: &str, attr: Attrs, color: Colors) -> String {
    format!("\u{1b}[{attr};{color}m{arg}\u{1b}[0m")
}

#[macro_export]
macro_rules! fancy_debug {
    ($($arg:tt)*) => {{
        println!(
            "\u{1b}[{};{}m>>{}\u{1b}[0m",
            $crate::fancy::Attrs::Inverse,
            $crate::fancy::Colors::White,
            format_args!($($arg)*)
        );
    }}
}

#[macro_export]
macro_rules! fancy_error {
    ($($arg:tt)*) => {{
        println!(
            "\u{1b}[{};{}m>>{}\u{1b}[0m",
            $crate::fancy::Attrs::Bold,
            $crate::fancy::Colors::Red,
            format_args!($($arg)*)
        );
    }}
}

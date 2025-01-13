use std::{fs::File, process::Command, process::Stdio};
use std::os::windows::io::IntoRawHandle;
use crate::{Outcome, Parameters, bad, shiplog};

pub fn daemon(func: fn(&Parameters) -> Outcome<()>, params: &Parameters) -> Outcome<()> {
    let exe = std::env::current_exe().expect("Failed to get current executable");
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    Command::new(exe)
        .args(&args)
        .creation_flags(0x00000008 | 0x00000010) // CREATE_NO_WINDOW | DETACHED_PROCESS
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| bad!(format!("Failed to daemonize: {}", e)))
        .and_then(|_| Ok(()))
}

fn redirect_stdio_to_null() {
    let devnull = File::open("NUL").expect("Failed to open NUL");
    unsafe {
        let devnull_handle = devnull.into_raw_handle();
        libc::SetStdHandle(libc::STD_INPUT_HANDLE, devnull_handle);
        libc::SetStdHandle(libc::STD_OUTPUT_HANDLE, devnull_handle);
        libc::SetStdHandle(libc::STD_ERROR_HANDLE, devnull_handle);
    }
}

use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::IntoRawHandle;
use std::{ffi::OsStr, fs::File, process::Command, process::Stdio};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Console::{
    SetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows::Win32::System::Threading::{
    CreateProcessW, CREATE_NO_WINDOW, DETACHED_PROCESS, PROCESS_CREATION_FLAGS,
    PROCESS_INFORMATION, STARTUPINFOW,
};

use crate::{bad, shiplog, Outcome, Parameters};

pub fn daemon(func: fn(&Parameters) -> Outcome<()>, params: &Parameters) -> Outcome<()> {
    let exe = std::env::current_exe().expect("Failed to get current executable");
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Convert the executable and arguments to wide strings
    let exe_wide: Vec<u16> = OsStr::new(&exe).encode_wide().chain(Some(0)).collect();
    let args_wide: Vec<u16> = OsStr::new(&args.join(" "))
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        let mut startup_info = STARTUPINFOW::default();
        let mut process_info = PROCESS_INFORMATION::default();
        let creation_flags = PROCESS_CREATION_FLAGS(CREATE_NO_WINDOW.0 | DETACHED_PROCESS.0);

        if let Err(e) = CreateProcessW(
            PCWSTR(exe_wide.as_ptr()),                 // Application name
            Some(PWSTR(args_wide.as_ptr() as *mut _)), // Command line
            None,                                      // Process security attributes
            None,                                      // Thread security attributes
            false,                                     // Inherit handles
            creation_flags,                            // Creation flags
            None,                                      // Environment block
            None,                                      // Current directory
            &mut startup_info,                         // Startup info
            &mut process_info,                         // Process info
        ) {
            return bad!("Failed to daemonize process");
        }

        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);
    }

    Ok(())
}

pub fn redirect_stdio_to_null() {
    let devnull = File::open("NUL").expect("Failed to open NUL");
    unsafe {
        let devnull_handle: HANDLE = HANDLE(devnull.into_raw_handle() as *mut _);
        SetStdHandle(STD_INPUT_HANDLE, devnull_handle).expect("Failed to set STD_INPUT_HANDLE");
        SetStdHandle(STD_OUTPUT_HANDLE, devnull_handle).expect("Failed to set STD_OUTPUT_HANDLE");
        SetStdHandle(STD_ERROR_HANDLE, devnull_handle).expect("Failed to set STD_ERROR_HANDLE");
    }
}

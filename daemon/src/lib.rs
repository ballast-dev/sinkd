use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::IntoRawHandle;
use std::{ffi::OsStr, fs::File};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Console::{
    SetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows::Win32::System::Threading::{
    CreateProcessW, CREATE_NO_WINDOW, DETACHED_PROCESS, PROCESS_CREATION_FLAGS,
    PROCESS_INFORMATION, STARTUPINFOW,
};

/// A convenience type for results in this crate.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Attempts to daemonize the current process on Windows 
/// by spawning a new detached child process 
/// flag: defaults to `--windows-daemon`.
pub fn daemon(flag: Option<&str>) -> Result<u32> {
    let exe = std::env::current_exe()?;
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(flag_str) = flag {
        args.push(flag_str.to_string());
    } else {
        args.push("--windows-daemon".to_string());
    }

    // Convert the executable and arguments to wide strings
    let exe_wide: Vec<u16> = OsStr::new(&exe).encode_wide().chain(Some(0)).collect();
    let args_wide: Vec<u16> = OsStr::new(&args.join(" ")).encode_wide().chain(Some(0)).collect();

    unsafe {
        let mut startup_info = STARTUPINFOW::default();
        let mut process_info = PROCESS_INFORMATION::default();
        let creation_flags = PROCESS_CREATION_FLAGS(CREATE_NO_WINDOW.0 | DETACHED_PROCESS.0);

        CreateProcessW(
            PCWSTR(exe_wide.as_ptr()),
            Some(PWSTR(args_wide.as_ptr() as *mut _)),
            None,  // Process security attributes
            None,  // Thread security attributes
            false, // Inherit handles
            creation_flags,
            None,  // Environment block
            None,  // Current directory
            &mut startup_info,
            &mut process_info,
        )
        .map_err(|e| format!("Failed to daemonize process: {e:?}"))?;

        // Clean up process handles
        CloseHandle(process_info.hProcess)
            .map_err(|e| format!("Failed to close hProcess: {e:?}"))?;
        CloseHandle(process_info.hThread)
            .map_err(|e| format!("Failed to close hThread: {e:?}"))?;

        Ok(process_info.dwProcessId)
    }
}

/// Redirects stdin, stdout, stderr to NUL (basically /dev/null on Windows).
pub fn redirect_stdio_to_null() -> Result<()> {
    let devnull = File::open("NUL")?;
    let devnull_handle: HANDLE = HANDLE(devnull.into_raw_handle() as *mut _);

    unsafe {
        SetStdHandle(STD_INPUT_HANDLE, devnull_handle)
            .map_err(|e| format!("Failed to set STD_INPUT_HANDLE: {e:?}"))?;
        SetStdHandle(STD_OUTPUT_HANDLE, devnull_handle)
            .map_err(|e| format!("Failed to set STD_OUTPUT_HANDLE: {e:?}"))?;
        SetStdHandle(STD_ERROR_HANDLE, devnull_handle)
            .map_err(|e| format!("Failed to set STD_ERROR_HANDLE: {e:?}"))?;
    }

    Ok(())
}
#[cfg(windows)]
mod platform {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE, FALSE};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        VK_RETURN, VK_UP, VK_DOWN, VK_ESCAPE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, SetWindowTextW,
        SetForegroundWindow, BringWindowToTop, ShowWindow,
        SW_RESTORE, IsWindowVisible,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
        CREATE_NEW_PROCESS_GROUP,
    };

    /// Launch a process with optional command-line arguments and return its process ID
    pub fn launch_process(exe_path: &Path, args: Option<&str>) -> Result<u32, String> {
        let exe_wide: Vec<u16> = OsStr::new(exe_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Build command line: "exe_path" args
        let cmd_line = match args {
            Some(a) => format!("\"{}\" {}", exe_path.display(), a),
            None => format!("\"{}\"", exe_path.display()),
        };
        let mut cmd_line_wide: Vec<u16> = OsStr::new(&cmd_line)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Set working directory to the exe's parent so it can find its config files
        let working_dir = exe_path.parent()
            .ok_or_else(|| format!("Cannot determine parent directory of {:?}", exe_path))?;
        let working_dir_wide: Vec<u16> = OsStr::new(working_dir)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut startup_info: STARTUPINFOW = unsafe { std::mem::zeroed() };
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

        let mut proc_info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let success = unsafe {
            CreateProcessW(
                exe_wide.as_ptr(),
                cmd_line_wide.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                FALSE,
                CREATE_NEW_PROCESS_GROUP,
                std::ptr::null(),
                working_dir_wide.as_ptr() as *const _,
                &startup_info,
                &mut proc_info,
            )
        };

        if success == FALSE {
            return Err(format!("CreateProcessW failed for {:?}", exe_path));
        }

        Ok(proc_info.dwProcessId)
    }

    /// Find a window by title prefix, returning its HWND
    pub fn find_window_by_title_prefix(prefix: &str) -> Option<HWND> {
        struct SearchData {
            prefix: Vec<u16>,
            result: Option<HWND>,
        }

        let prefix_wide: Vec<u16> = OsStr::new(prefix)
            .encode_wide()
            .collect();

        let mut data = SearchData {
            prefix: prefix_wide,
            result: None,
        };

        unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let data = &mut *(lparam as *mut SearchData);
            if IsWindowVisible(hwnd) == FALSE {
                return TRUE; // continue
            }
            let mut title = [0u16; 256];
            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 256);
            if len > 0 {
                let title_slice = &title[..len as usize];
                if title_slice.starts_with(&data.prefix) {
                    data.result = Some(hwnd);
                    return FALSE; // stop
                }
            }
            TRUE // continue
        }

        unsafe {
            EnumWindows(Some(callback), &mut data as *mut SearchData as LPARAM);
        }

        data.result
    }

    /// Find all windows matching a title prefix
    pub fn find_windows_by_title_prefix(prefix: &str) -> Vec<HWND> {
        struct SearchData {
            prefix: Vec<u16>,
            results: Vec<HWND>,
        }

        let prefix_wide: Vec<u16> = OsStr::new(prefix)
            .encode_wide()
            .collect();

        let mut data = SearchData {
            prefix: prefix_wide,
            results: Vec::new(),
        };

        unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let data = &mut *(lparam as *mut SearchData);
            if IsWindowVisible(hwnd) == FALSE {
                return TRUE;
            }
            let mut title = [0u16; 256];
            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 256);
            if len > 0 {
                let title_slice = &title[..len as usize];
                if title_slice.starts_with(&data.prefix) {
                    data.results.push(hwnd);
                }
            }
            TRUE
        }

        unsafe {
            EnumWindows(Some(callback), &mut data as *mut SearchData as LPARAM);
        }

        data.results
    }

    /// Set a window's title
    pub fn set_window_title(hwnd: HWND, title: &str) {
        let title_wide: Vec<u16> = OsStr::new(title)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            SetWindowTextW(hwnd, title_wide.as_ptr());
        }
    }

    /// Focus a window
    pub fn focus_window(hwnd: HWND) {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
        }
    }

    /// Simulate a key press (down + up) with configurable hold time
    pub fn press_key(vk: u16, hold_ms: u64) {
        let mut inputs: [INPUT; 2] = unsafe { std::mem::zeroed() };

        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki = KEYBDINPUT {
            wVk: vk,
            wScan: 0,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };

        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki = KEYBDINPUT {
            wVk: vk,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };

        unsafe {
            SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
        std::thread::sleep(std::time::Duration::from_millis(hold_ms));
    }

    /// Block or unblock user input (requires admin)
    pub fn block_input(block: bool) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::BlockInput;
        unsafe {
            BlockInput(if block { TRUE } else { FALSE });
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    // Stub type for HWND on non-Windows
    pub type HWND = isize;

    pub fn launch_process(exe_path: &Path, _args: Option<&str>) -> Result<u32, String> {
        log::warn!("launch_process is a stub on non-Windows: {:?}", exe_path);
        Ok(0)
    }

    pub fn find_window_by_title_prefix(_prefix: &str) -> Option<HWND> {
        log::warn!("find_window_by_title_prefix is a stub on non-Windows");
        None
    }

    pub fn find_windows_by_title_prefix(_prefix: &str) -> Vec<HWND> {
        log::warn!("find_windows_by_title_prefix is a stub on non-Windows");
        Vec::new()
    }

    pub fn set_window_title(_hwnd: HWND, _title: &str) {
        log::warn!("set_window_title is a stub on non-Windows");
    }

    pub fn focus_window(_hwnd: HWND) {
        log::warn!("focus_window is a stub on non-Windows");
    }

    pub fn press_key(_vk: u16, _hold_ms: u64) {
        log::warn!("press_key is a stub on non-Windows");
    }

    pub fn block_input(_block: bool) {
        log::warn!("block_input is a stub on non-Windows");
    }
}

pub use platform::*;

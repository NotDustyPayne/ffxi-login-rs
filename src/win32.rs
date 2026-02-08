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

    /// Type text character by character using SendInput and VkKeyScan
    pub fn type_text(text: &str) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::VkKeyScanA;

        for ch in text.bytes() {
            let result = unsafe { VkKeyScanA(ch as i8) };
            let vk = (result & 0xFF) as u16;
            let shift_state = ((result >> 8) & 0xFF) as u16;

            let needs_shift = shift_state & 1 != 0;
            let vk_shift: u16 = 0x10;

            if needs_shift {
                // Shift down
                let mut shift_down: [INPUT; 1] = unsafe { std::mem::zeroed() };
                shift_down[0].r#type = INPUT_KEYBOARD;
                shift_down[0].Anonymous.ki = KEYBDINPUT {
                    wVk: vk_shift, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0,
                };
                unsafe { SendInput(1, shift_down.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
            }

            // Key down + up
            let mut inputs: [INPUT; 2] = unsafe { std::mem::zeroed() };
            inputs[0].r#type = INPUT_KEYBOARD;
            inputs[0].Anonymous.ki = KEYBDINPUT {
                wVk: vk, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0,
            };
            inputs[1].r#type = INPUT_KEYBOARD;
            inputs[1].Anonymous.ki = KEYBDINPUT {
                wVk: vk, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0,
            };
            unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }

            if needs_shift {
                // Shift up
                let mut shift_up: [INPUT; 1] = unsafe { std::mem::zeroed() };
                shift_up[0].r#type = INPUT_KEYBOARD;
                shift_up[0].Anonymous.ki = KEYBDINPUT {
                    wVk: vk_shift, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0,
                };
                unsafe { SendInput(1, shift_up.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// Paste text by setting the clipboard and pressing Ctrl+V
    pub fn paste_text(text: &str) {
        use windows_sys::Win32::System::DataExchange::{
            OpenClipboard, CloseClipboard, EmptyClipboard, SetClipboardData,
        };
        use windows_sys::Win32::System::Memory::{
            GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
        };
        use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

        let wide: Vec<u16> = OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let size = wide.len() * 2;

        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == FALSE {
                return;
            }
            EmptyClipboard();

            let hmem = GlobalAlloc(GMEM_MOVEABLE, size);
            if hmem.is_null() {
                CloseClipboard();
                return;
            }

            let ptr = GlobalLock(hmem);
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, size);
                GlobalUnlock(hmem);
            }

            SetClipboardData(CF_UNICODETEXT as u32, hmem as _);
            CloseClipboard();
        }

        // Press Ctrl+V
        let vk_control: u16 = 0x11;
        let vk_v: u16 = 0x56;

        let mut inputs: [INPUT; 4] = unsafe { std::mem::zeroed() };

        // Ctrl down
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki = KEYBDINPUT {
            wVk: vk_control, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0,
        };
        // V down
        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki = KEYBDINPUT {
            wVk: vk_v, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0,
        };
        // V up
        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki = KEYBDINPUT {
            wVk: vk_v, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0,
        };
        // Ctrl up
        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki = KEYBDINPUT {
            wVk: vk_control, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0,
        };

        unsafe {
            SendInput(4, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
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

    pub fn type_text(_text: &str) {
        log::warn!("type_text is a stub on non-Windows");
    }

    pub fn paste_text(_text: &str) {
        log::warn!("paste_text is a stub on non-Windows");
    }

    pub fn block_input(_block: bool) {
        log::warn!("block_input is a stub on non-Windows");
    }
}

pub use platform::*;

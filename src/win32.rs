#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyDirection {
    Down,
    Up,
}

#[derive(Debug, Clone)]
pub struct RecordedKey {
    pub vk_code: u16,
    pub direction: KeyDirection,
    pub delay_ms: u64,
}

#[cfg(windows)]
mod platform {
    use super::{KeyDirection, RecordedKey};
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::sync::Mutex;
    use std::time::Instant;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM, TRUE, FALSE};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        VK_RETURN,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, SetWindowTextW,
        SetForegroundWindow, BringWindowToTop, ShowWindow,
        SW_RESTORE, IsWindowVisible,
        SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx,
        GetMessageW, MSG, WH_KEYBOARD_LL, KBDLLHOOKSTRUCT,
        WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
        CREATE_NEW_PROCESS_GROUP,
    };
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows_sys::Win32::Security::{TOKEN_QUERY, TokenElevation, TOKEN_ELEVATION, GetTokenInformation};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    /// Check if the current process is running with admin privileges
    pub fn is_elevated() -> bool {
        unsafe {
            let mut token = std::mem::zeroed();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == FALSE {
                return false;
            }

            let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
            let mut size = 0u32;
            let result = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );

            windows_sys::Win32::Foundation::CloseHandle(token);
            result != FALSE && elevation.TokenIsElevated != 0
        }
    }

    /// Re-launch the current process elevated via UAC, then exit
    pub fn elevate_self() -> ! {
        let exe = std::env::current_exe()
            .expect("Failed to get current exe path");
        let args: Vec<String> = std::env::args().skip(1).collect();
        let args_str = args.join(" ");

        let verb: Vec<u16> = OsStr::new("runas")
            .encode_wide().chain(std::iter::once(0)).collect();
        let exe_wide: Vec<u16> = OsStr::new(&exe)
            .encode_wide().chain(std::iter::once(0)).collect();
        let args_wide: Vec<u16> = OsStr::new(&args_str)
            .encode_wide().chain(std::iter::once(0)).collect();
        let cwd = std::env::current_dir()
            .expect("Failed to get current directory");
        let cwd_wide: Vec<u16> = OsStr::new(&cwd)
            .encode_wide().chain(std::iter::once(0)).collect();

        let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.fMask = SEE_MASK_NOCLOSEPROCESS;
        sei.lpVerb = verb.as_ptr();
        sei.lpFile = exe_wide.as_ptr();
        sei.lpParameters = args_wide.as_ptr();
        sei.lpDirectory = cwd_wide.as_ptr();
        sei.nShow = SW_SHOWNORMAL;

        let success = unsafe { ShellExecuteExW(&mut sei) };
        if success == FALSE {
            eprintln!("Failed to elevate. Please run as administrator.");
            std::process::exit(1);
        }

        std::process::exit(0);
    }

    /// Launch a process with optional command-line arguments and return its process ID
    pub fn launch_process(exe_path: &Path, args: Option<&str>) -> Result<u32, String> {
        let exe_wide: Vec<u16> = OsStr::new(exe_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let cmd_line = match args {
            Some(a) => format!("\"{}\" {}", exe_path.display(), a),
            None => format!("\"{}\"", exe_path.display()),
        };
        let mut cmd_line_wide: Vec<u16> = OsStr::new(&cmd_line)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

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
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            return Err(format!("CreateProcessW failed for {:?} (error code {})", exe_path, err));
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

    // --- Keyboard recording ---

    struct RecorderState {
        last_time: Option<Instant>,
        index: usize,
    }

    static RECORDER: Mutex<Option<RecorderState>> = Mutex::new(None);

    unsafe extern "system" fn keyboard_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            let kb = &*(l_param as *const KBDLLHOOKSTRUCT);
            let vk = kb.vkCode as u16;
            let is_down = w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN;
            let is_up = w_param as u32 == WM_KEYUP || w_param as u32 == WM_SYSKEYUP;

            if is_down || is_up {
                if let Ok(mut guard) = RECORDER.lock() {
                    if let Some(state) = guard.as_mut() {
                        let direction = if is_down { "DOWN" } else { "UP" };

                        let now = Instant::now();
                        let delay_ms = state
                            .last_time
                            .map(|t| now.duration_since(t).as_millis() as u64)
                            .unwrap_or(0);
                        state.last_time = Some(now);

                        println!(
                            "{:<6} {:<20} {:<6} +{}ms",
                            state.index,
                            super::vk_name(vk),
                            direction,
                            delay_ms,
                        );

                        state.index += 1;
                    }
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
    }

    /// Stream keyboard events to stdout using a low-level hook. Runs until the process exits (Ctrl+C).
    pub fn record_keys_stream() {
        // Initialize recorder state
        {
            let mut guard = RECORDER.lock().unwrap();
            *guard = Some(RecorderState {
                last_time: None,
                index: 0,
            });
        }

        // Install hook
        let hook = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), std::ptr::null_mut(), 0)
        };
        if hook.is_null() {
            eprintln!("Failed to install keyboard hook");
            return;
        }

        // Run message loop forever (Ctrl+C to exit)
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                // Just pump messages; the hook callback does all the work
            }

            UnhookWindowsHookEx(hook);
        }
    }

    /// Replay recorded keyboard events with their original timing.
    pub fn replay_keys(keys: &[RecordedKey]) {
        for key in keys {
            if key.delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(key.delay_ms));
            }

            let flags = match key.direction {
                KeyDirection::Down => 0u32,
                KeyDirection::Up => KEYEVENTF_KEYUP,
            };

            let mut input: [INPUT; 1] = unsafe { std::mem::zeroed() };
            input[0].r#type = INPUT_KEYBOARD;
            input[0].Anonymous.ki = KEYBDINPUT {
                wVk: key.vk_code,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };

            unsafe {
                SendInput(1, input.as_ptr(), std::mem::size_of::<INPUT>() as i32);
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::RecordedKey;
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

    pub fn record_keys_stream() {
        log::warn!("record_keys_stream is a stub on non-Windows");
    }

    pub fn replay_keys(_keys: &[RecordedKey]) {
        log::warn!("replay_keys is a stub on non-Windows");
    }
}

pub use platform::*;

pub fn vk_name(vk: u16) -> String {
    match vk {
        0x08 => "BACKSPACE".into(),
        0x09 => "TAB".into(),
        0x0D => "ENTER".into(),
        0x10 => "SHIFT".into(),
        0x11 => "CTRL".into(),
        0x12 => "ALT".into(),
        0x14 => "CAPS_LOCK".into(),
        0x1B => "ESCAPE".into(),
        0x20 => "SPACE".into(),
        0x21 => "PAGE_UP".into(),
        0x22 => "PAGE_DOWN".into(),
        0x23 => "END".into(),
        0x24 => "HOME".into(),
        0x25 => "LEFT".into(),
        0x26 => "UP".into(),
        0x27 => "RIGHT".into(),
        0x28 => "DOWN".into(),
        0x2D => "INSERT".into(),
        0x2E => "DELETE".into(),
        0x30..=0x39 => format!("{}", vk - 0x30),
        0x41..=0x5A => format!("{}", vk as u8 as char),
        0x60..=0x69 => format!("NUMPAD_{}", vk - 0x60),
        0x6A => "MULTIPLY".into(),
        0x6B => "ADD".into(),
        0x6D => "SUBTRACT".into(),
        0x6E => "DECIMAL".into(),
        0x6F => "DIVIDE".into(),
        0x70..=0x7B => format!("F{}", vk - 0x70 + 1),
        0xA0 => "LSHIFT".into(),
        0xA1 => "RSHIFT".into(),
        0xA2 => "LCTRL".into(),
        0xA3 => "RCTRL".into(),
        0xA4 => "LALT".into(),
        0xA5 => "RALT".into(),
        _ => format!("VK_0x{:02X}", vk),
    }
}

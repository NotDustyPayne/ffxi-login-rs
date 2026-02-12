use crate::config::{Character, Config};
use crate::hosts;
use crate::logging::FileLogger;
use crate::proxy;
use crate::win32;
use std::thread;
use std::time::Duration;

#[cfg(windows)]
type WindowHandle = windows_sys::Win32::Foundation::HWND;
#[cfg(not(windows))]
type WindowHandle = isize;

struct LaunchedCharacter<'a> {
    character: &'a Character,
    hwnd: WindowHandle,
}

pub fn run(config: &Config, characters: &[&Character], logger: &FileLogger) {
    // Clean any stale hosts entries from a previous crash
    hosts::cleanup_stale();

    // Phase 1: Launch all Windower instances
    println!("\n=== Phase 1: Launching Windower instances ===");
    let launched = phase1_launch(config, characters, logger);

    if launched.is_empty() {
        eprintln!("No Windower instances launched successfully. Aborting.");
        return;
    }

    // Brief pause to let all POL windows initialize
    println!("Waiting for PlayOnline windows to initialize...");
    thread::sleep(Duration::from_secs(5));

    // Phase 2: Automate POL login for each
    println!("\n=== Phase 2: Automating PlayOnline login ===");
    phase2_login(config, &launched, logger);

    println!("\n=== Done ===");
}

fn phase1_launch<'a>(
    config: &Config,
    characters: &[&'a Character],
    logger: &FileLogger,
) -> Vec<LaunchedCharacter<'a>> {
    let mut launched = Vec::new();

    // Track existing POL windows so we can identify new ones
    let existing_windows = win32::find_windows_by_title_prefix("PlayOnline Viewer");

    for (i, character) in characters.iter().enumerate() {
        println!("  Launching Windower for {}...", character.name);

        match launch_with_retry(config, character, &existing_windows, &launched, logger) {
            Some(lc) => {
                println!("  ✓ {} - window found", character.name);
                launched.push(lc);
            }
            None => {
                println!("  ✗ {} - failed after retry", character.name);
            }
        }

        // Small delay between launches
        if i < characters.len() - 1 {
            thread::sleep(Duration::from_secs(config.launch_delay_seconds));
        }
    }

    launched
}

fn launch_with_retry<'a>(
    config: &Config,
    character: &'a Character,
    existing_windows: &[WindowHandle],
    already_launched: &[LaunchedCharacter],
    logger: &FileLogger,
) -> Option<LaunchedCharacter<'a>> {
    for attempt in 0..2 {
        if attempt > 0 {
            log::info!("Retrying launch for {}", character.name);
        }

        match launch_single(config, character, existing_windows, already_launched) {
            Ok(lc) => return Some(lc),
            Err(e) => {
                let step = if attempt == 0 { "launch" } else { "launch (retry)" };
                logger.log_error(&character.name, step, &e);
            }
        }
    }

    None
}

fn launch_single<'a>(
    config: &Config,
    character: &'a Character,
    existing_windows: &[WindowHandle],
    already_launched: &[LaunchedCharacter],
) -> Result<LaunchedCharacter<'a>, String> {
    // Launch Windower with -p flag to skip the profile picker
    let profile_arg = config.windower_profile.as_ref()
        .map(|p| format!("-p=\"{}\"", p));
    let _pid = win32::launch_process(
        &config.windower_path,
        profile_arg.as_deref(),
    )?;

    // Poll for new PlayOnline window (up to 30 seconds)
    let mut new_hwnd = None;
    for _ in 0..60 {
        thread::sleep(Duration::from_millis(500));

        let all_windows = win32::find_windows_by_title_prefix("PlayOnline Viewer");
        for hwnd in &all_windows {
            let is_existing = existing_windows.contains(hwnd);
            let is_already_launched = already_launched.iter().any(|lc| lc.hwnd == *hwnd);
            if !is_existing && !is_already_launched {
                new_hwnd = Some(*hwnd);
                break;
            }
        }
        if new_hwnd.is_some() {
            break;
        }
    }

    let hwnd = new_hwnd.ok_or_else(|| {
        format!("Timed out waiting for PlayOnline window for {}", character.name)
    })?;

    // Rename window for identification
    let title = format!("PlayOnline Viewer - {}", character.name);
    win32::set_window_title(hwnd, &title);

    Ok(LaunchedCharacter { character, hwnd })
}

fn phase2_login(config: &Config, launched: &[LaunchedCharacter], logger: &FileLogger) {
    let port = config.region.proxy_port();

    for (i, lc) in launched.iter().enumerate() {
        println!("  Logging in {}...", lc.character.name);

        match login_with_retry(config, lc, port, logger) {
            Ok(()) => println!("  ✓ {} - login automation complete", lc.character.name),
            Err(()) => println!("  ✗ {} - login failed after retry", lc.character.name),
        }

        // Stagger delay before next character
        if i < launched.len() - 1 {
            println!(
                "  Waiting {} seconds before next login...",
                config.stagger_delay_seconds
            );
            thread::sleep(Duration::from_secs(config.stagger_delay_seconds));
        }
    }
}

fn login_with_retry(
    config: &Config,
    lc: &LaunchedCharacter,
    port: u16,
    logger: &FileLogger,
) -> Result<(), ()> {
    loop {
        match login_single(config, lc, port) {
            Ok(()) => return Ok(()),
            Err(e) => {
                logger.log_error(&lc.character.name, "login", &e);
                // Always cleanup on failure
                win32::block_input(false);
                let _ = hosts::remove_entries();

                println!("  ✗ {} login failed: {}", lc.character.name, e);
                println!("  Press Enter to retry, or type 'skip' to skip this character:");

                let mut input = String::new();
                if std::io::stdin().read_line(&mut input).is_err() {
                    return Err(());
                }
                if input.trim().eq_ignore_ascii_case("skip") {
                    return Err(());
                }
            }
        }
    }
}

fn login_single(
    config: &Config,
    lc: &LaunchedCharacter,
    port: u16,
) -> Result<(), String> {
    // Start proxy server
    let proxy_handle = proxy::start_proxy(port)
        .map_err(|e| format!("Failed to start proxy: {}", e))?;

    // Add hosts entry
    hosts::add_entry(config.region.hosts_entry())
        .map_err(|e| format!("Failed to add hosts entry: {}", e))?;

    // Block user input
    win32::block_input(true);

    // Focus the POL window, position cursor in neutral zone, then scroll
    // up to guarantee slot 1 is selected before navigating down.
    println!("    {}: targeting slot {}", lc.character.name, lc.character.slot);
    win32::focus_window(lc.hwnd);
    win32::move_cursor_to_window(lc.hwnd);
    thread::sleep(Duration::from_millis(500));

    // Scroll mouse wheel up to reset to slot 1
    println!("    Resetting slot position with mouse wheel up");
    for _ in 0..20 {
        win32::mouse_scroll_up();
    }
    thread::sleep(Duration::from_millis(300));

    // Navigate to target slot: press DOWN `slot` times (first DOWN activates slot 1)
    let down_presses = lc.character.slot;
    println!("    Moving down {} time(s) to reach slot {}", down_presses, lc.character.slot);
    for i in 0..down_presses {
        println!("    DOWN press {}/{}", i + 1, down_presses);
        win32::press_key(0x28, 200);
    }

    // Select the slot
    println!("    ENTER to select slot");
    win32::press_key(0x0D, 300);
    thread::sleep(Duration::from_millis(1500));

    // Step 2: First confirmation screen
    log::debug!("ENTER (confirmation screen 1)");
    win32::press_key(0x0D, 300);
    thread::sleep(Duration::from_millis(1500));

    // Step 3: Second confirmation screen
    log::debug!("ENTER (confirmation screen 2)");
    win32::press_key(0x0D, 300);
    thread::sleep(Duration::from_millis(1500));

    // Step 4: Third confirmation screen
    log::debug!("ENTER (confirmation screen 3)");
    win32::press_key(0x0D, 300);
    thread::sleep(Duration::from_millis(1500));

    // Step 5: Navigate to password input field (UP, RIGHT, RIGHT, ENTER)
    log::debug!("UP, RIGHT, RIGHT, ENTER (navigate to password field)");
    win32::press_key(0x26, 150);
    win32::press_key(0x27, 150);
    win32::press_key(0x27, 150);
    win32::press_key(0x0D, 150);
    thread::sleep(Duration::from_millis(500));

    // Step 6: Type password
    log::debug!("typing password ({} chars)", lc.character.password.len());
    win32::type_text(&lc.character.password);
    thread::sleep(Duration::from_millis(300));

    // Step 7: Submit password
    log::debug!("ENTER (submit password)");
    win32::press_key(0x0D, 150);
    thread::sleep(Duration::from_millis(500));

    // Step 8: Navigate to Connect and press it
    log::debug!("DOWN, ENTER (connect)");
    win32::press_key(0x28, 150);
    win32::press_key(0x0D, 150);
    thread::sleep(Duration::from_millis(500));

    // Unblock user input
    win32::block_input(false);

    // Wait for proxy to serve its response
    let _ = proxy_handle.join();

    // Remove hosts entry
    hosts::remove_entries()
        .map_err(|e| format!("Failed to remove hosts entry: {}", e))?;

    Ok(())
}

pub fn run_record_mode(config: &Config, characters: &[&Character], logger: &FileLogger) {
    let character = characters[0];
    println!("\n=== Record Mode ===");
    println!("Launching Windower for {}...", character.name);

    let existing_windows = win32::find_windows_by_title_prefix("PlayOnline Viewer");

    match launch_single(config, character, &existing_windows, &[]) {
        Ok(lc) => {
            println!("PlayOnline window found for {}", character.name);
            println!("Waiting for PlayOnline to initialize...");
            thread::sleep(Duration::from_secs(5));

            win32::focus_window(lc.hwnd);
            thread::sleep(Duration::from_millis(500));

            println!();
            println!("=== Recording keypresses (Ctrl+C to stop) ===");
            println!("Manually perform the login in PlayOnline.");
            println!();
            println!("{:<6} {:<20} {:<6} Delay", "#", "Key", "Dir");
            println!("{}", "-".repeat(80));

            // Streams events to stdout until Ctrl+C
            // Mouse positions are logged relative to the POL window
            win32::record_keys_stream(lc.hwnd);
        }
        Err(e) => {
            logger.log_error(&character.name, "record_launch", &e);
            eprintln!("Failed to launch: {}", e);
        }
    }
}

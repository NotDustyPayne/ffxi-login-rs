// Unused for now — slot navigation uses cursor-reset approach instead.
// Kept for potential future use (auto-login detection, etc.)
#![allow(dead_code)]

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const SLOT_OFFSET: u64 = 0x64;
const AUTO_LOGIN_OFFSET: u64 = 0x6F;

pub struct LoginBinInfo {
    pub current_slot: u8,
    pub auto_login_enabled: bool,
}

/// Read login_w.bin to get the currently selected slot and auto-login status
pub fn read_login_bin(playonline_dir: &Path) -> Result<LoginBinInfo, Box<dyn std::error::Error>> {
    let path = playonline_dir.join("login_w.bin");
    let mut file = File::open(&path)
        .map_err(|e| format!("Failed to open {:?}: {}", path, e))?;

    let mut slot_byte = [0u8; 1];
    file.seek(SeekFrom::Start(SLOT_OFFSET))?;
    file.read_exact(&mut slot_byte)?;

    let mut auto_login_byte = [0u8; 1];
    file.seek(SeekFrom::Start(AUTO_LOGIN_OFFSET))?;
    file.read_exact(&mut auto_login_byte)?;

    let info = LoginBinInfo {
        current_slot: slot_byte[0],
        auto_login_enabled: auto_login_byte[0] != 0,
    };

    log::info!(
        "login_w.bin: current_slot={}, auto_login={}",
        info.current_slot,
        info.auto_login_enabled
    );

    Ok(info)
}

/// Calculate navigation steps from current slot to target slot
pub fn navigation_steps(current: u8, target: u8) -> Vec<NavDirection> {
    let diff = target as i16 - current as i16;
    if diff > 0 {
        vec![NavDirection::Down; diff as usize]
    } else if diff < 0 {
        vec![NavDirection::Up; (-diff) as usize]
    } else {
        Vec::new()
    }
}

#[derive(Debug, Clone)]
pub enum NavDirection {
    Up,
    Down,
}

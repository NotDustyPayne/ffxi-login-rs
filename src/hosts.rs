use std::fs;
use std::io::Write;

const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";
const MARKER: &str = "# ffxi-login-rs";

/// Add the hosts file entry for POL redirect
pub fn add_entry(entry: &str) -> Result<(), Box<dyn std::error::Error>> {
    let line = format!("{} {}\n", entry, MARKER);

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(HOSTS_PATH)?;

    file.write_all(line.as_bytes())?;
    log::info!("Added hosts entry: {}", entry);
    Ok(())
}

/// Remove all ffxi-login-rs entries from the hosts file
pub fn remove_entries() -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(HOSTS_PATH)?;
    let filtered: Vec<&str> = contents
        .lines()
        .filter(|line| !line.contains(MARKER))
        .collect();

    fs::write(HOSTS_PATH, filtered.join("\n") + "\n")?;
    log::info!("Removed hosts entries");
    Ok(())
}

/// Ensure cleanup happens even on unexpected exit.
/// Call this at startup to remove any stale entries from a previous crash.
pub fn cleanup_stale() {
    if let Err(e) = remove_entries() {
        log::warn!("Could not clean stale hosts entries: {}", e);
    }
}

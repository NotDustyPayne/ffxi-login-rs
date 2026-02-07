use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub struct FileLogger {
    log_dir: PathBuf,
}

impl FileLogger {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let log_dir = std::env::temp_dir().join("login-rs");

        // Clear previous logs on startup
        if log_dir.exists() {
            fs::remove_dir_all(&log_dir)?;
        }
        fs::create_dir_all(&log_dir)?;

        log::info!("Log directory: {}", log_dir.display());
        Ok(Self { log_dir })
    }

    pub fn log_error(&self, character_name: &str, step: &str, error: &str) {
        let filename = format!("{}.log", character_name);
        let path = self.log_dir.join(&filename);

        let timestamp = chrono_free_timestamp();
        let entry = format!("[{}] Step: {} | Error: {}\n", timestamp, step, error);

        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = file.write_all(entry.as_bytes());
        }

        log::error!("[{}] {}: {}", character_name, step, error);
    }

    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
}

fn chrono_free_timestamp() -> String {
    // Use SystemTime to avoid adding chrono dependency
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("{}", d.as_secs()),
        Err(_) => "unknown".to_string(),
    }
}

use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_WINDOWER_PATH: &str = r"C:\Windower4\Windower.exe";
const DEFAULT_PLAYONLINE_DIR: &str =
    r"C:\Program Files (x86)\PlayOnline\SquareEnix\PlayOnlineViewer\usr\all";

fn default_windower_path() -> PathBuf {
    PathBuf::from(DEFAULT_WINDOWER_PATH)
}

fn default_playonline_dir() -> PathBuf {
    PathBuf::from(DEFAULT_PLAYONLINE_DIR)
}

fn default_stagger_delay() -> u64 {
    10
}

fn default_launch_delay() -> u64 {
    2
}

fn default_region() -> Region {
    Region::Us
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_windower_path")]
    pub windower_path: PathBuf,
    #[serde(default = "default_playonline_dir")]
    pub playonline_dir: PathBuf,
    pub windower_profile: Option<String>,
    #[serde(default = "default_stagger_delay")]
    pub stagger_delay_seconds: u64,
    #[serde(default = "default_launch_delay")]
    pub launch_delay_seconds: u64,
    #[serde(default = "default_region")]
    pub region: Region,
    pub characters: Vec<Character>,
}

#[derive(Debug, Deserialize)]
pub struct Character {
    pub name: String,
    pub slot: u8,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    Us,
    Jp,
    Eu,
}

impl Region {
    pub fn proxy_port(&self) -> u16 {
        match self {
            Region::Us => 51304,
            Region::Jp => 51300,
            Region::Eu => 51302,
        }
    }

    pub fn hosts_entry(&self) -> &'static str {
        // All regions use the same hostname pattern
        "127.0.0.1 wh000.pol.com"
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.characters.is_empty() {
            return Err("No characters defined in config".into());
        }
        for ch in &self.characters {
            if ch.slot < 1 || ch.slot > 20 {
                return Err(format!(
                    "Character '{}' has invalid slot {} (must be 1-20)",
                    ch.name, ch.slot
                )
                .into());
            }
        }
        if self.stagger_delay_seconds == 0 {
            return Err("stagger_delay_seconds must be > 0".into());
        }
        Ok(())
    }

    pub fn filter_characters(&self, names: &[String]) -> Vec<&Character> {
        if names.is_empty() {
            self.characters.iter().collect()
        } else {
            self.characters
                .iter()
                .filter(|c| names.iter().any(|n| n.eq_ignore_ascii_case(&c.name)))
                .collect()
        }
    }
}

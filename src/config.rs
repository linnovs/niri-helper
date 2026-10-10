use std::env;
use std::path::PathBuf;

use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub pin: Pin,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Pin {
    pub app_id: Vec<String>,
}

impl Config {
    pub fn load() -> std::io::Result<Self> {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid config at {}: {e}", path.display()),
                )
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn app_id_patterns(&self) -> Result<Vec<Regex>, regex::Error> {
        compile(&self.pin.app_id)
    }
}

fn compile(patterns: &[String]) -> Result<Vec<Regex>, regex::Error> {
    patterns.iter().map(|p| Regex::new(p)).collect()
}

fn config_path() -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("niri-helper").join("config.toml")
}

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub pin: Pin,
    pub scratchpad: Scratchpad,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Pin {
    pub app_id: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub app_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Scratchpad {
    pub workspace: String,
    pub applications: HashMap<String, Application>,
}

impl Default for Scratchpad {
    fn default() -> Self {
        Self {
            workspace: "scratch".to_string(),
            applications: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Application {
    pub app_id: String,
    pub executable: String,
}

/// A compiled rule: both regexes must match for the rule to match a window.
#[derive(Debug, Clone)]
pub struct CompiledRule {
    app_id: Regex,
    title: Regex,
}

impl CompiledRule {
    pub fn matches(&self, app_id: Option<&str>, title: Option<&str>) -> bool {
        app_id.is_some_and(|a| self.app_id.is_match(a))
            && title.is_some_and(|t| self.title.is_match(t))
    }
}

/// The reloadable subset of the config the daemon operates on.
#[derive(Debug, Clone)]
pub struct ConfigData {
    pub config: Config,
    pub pin_patterns: Vec<Regex>,
    pub scratchpad_patterns: Vec<Regex>,
    pub rule_matchers: Vec<CompiledRule>,
}

impl ConfigData {
    pub fn load() -> std::io::Result<Self> {
        let config = Config::load()?;
        let pin_patterns = config.app_id_patterns().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid pin.app_id regex: {e}"),
            )
        })?;
        let scratchpad_patterns = config.scratchpad_patterns().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid scratchpad application regex: {e}"),
            )
        })?;
        let rule_matchers = config.rule_matchers().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid rules regex: {e}"),
            )
        })?;
        Ok(Self {
            config,
            pin_patterns,
            scratchpad_patterns,
            rule_matchers,
        })
    }
}

impl Config {
    pub fn load() -> std::io::Result<Self> {
        let path = config_path();
        match fs::read_to_string(&path) {
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

    pub fn scratchpad_patterns(&self) -> Result<Vec<Regex>, regex::Error> {
        self.scratchpad
            .applications
            .values()
            .map(|a| Regex::new(&a.app_id))
            .collect()
    }

    pub fn rule_matchers(&self) -> Result<Vec<CompiledRule>, regex::Error> {
        self.rules
            .iter()
            .map(|r| {
                Ok(CompiledRule {
                    app_id: Regex::new(&r.app_id)?,
                    title: Regex::new(&r.title)?,
                })
            })
            .collect()
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

/// Fingerprint (mtime + length) of the config file, for change detection.
pub fn config_fingerprint() -> Option<(SystemTime, u64)> {
    let meta = fs::metadata(config_path()).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

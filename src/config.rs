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
    pub size: Option<Size>,
    pub position: Option<Position>,
}

/// Optional absolute window size, in logical pixels.
#[derive(Debug, Clone, Deserialize)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

/// Optional window position. Each axis is either an absolute offset in logical
/// pixels (e.g. `100`) or a percentage of the working area (e.g. `"50%"`).
#[derive(Debug, Clone, Deserialize)]
pub struct Position {
    pub x: PositionValue,
    pub y: PositionValue,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionValue {
    Fixed(f64),
    Proportion(f64),
}

impl<'de> serde::Deserialize<'de> for PositionValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = toml::Value::deserialize(deserializer)?;
        match value {
            toml::Value::Integer(i) => Ok(PositionValue::Fixed(i as f64)),
            toml::Value::Float(f) => Ok(PositionValue::Fixed(f)),
            toml::Value::String(s) => {
                if let Some(percent) = s.strip_suffix('%') {
                    let value: f64 = percent.trim().parse().map_err(D::Error::custom)?;
                    Ok(PositionValue::Proportion(value))
                } else {
                    let value: f64 = s.trim().parse().map_err(D::Error::custom)?;
                    Ok(PositionValue::Fixed(value))
                }
            }
            other => Err(D::Error::custom(format!(
                "expected a number or \"NN%\" string, got {other:?}"
            ))),
        }
    }
}

impl Position {
    pub fn changes(&self) -> (niri_ipc::PositionChange, niri_ipc::PositionChange) {
        let to_change = |v: PositionValue| match v {
            PositionValue::Fixed(px) => niri_ipc::PositionChange::SetFixed(px),
            PositionValue::Proportion(p) => niri_ipc::PositionChange::SetProportion(p),
        };
        (to_change(self.x), to_change(self.y))
    }
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
    size: Option<Size>,
    position: Option<Position>,
}

impl CompiledRule {
    pub fn matches(&self, app_id: Option<&str>, title: Option<&str>) -> bool {
        app_id.is_some_and(|a| self.app_id.is_match(a))
            && title.is_some_and(|t| self.title.is_match(t))
    }

    pub fn size(&self) -> Option<&Size> {
        self.size.as_ref()
    }

    pub fn position(&self) -> Option<&Position> {
        self.position.as_ref()
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
                    size: r.size.clone(),
                    position: r.position.clone(),
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

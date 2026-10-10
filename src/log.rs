use std::env;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use chrono::Local;

const MAX_LOG_SIZE: u64 = 1024 * 1024;

#[derive(Clone, Copy)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        })
    }
}

pub struct Logger {
    path: PathBuf,
    file: File,
    size: u64,
}

impl Logger {
    pub fn init() -> io::Result<Self> {
        let path = log_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path, file, size })
    }

    pub fn log(&mut self, level: Level, args: fmt::Arguments<'_>) {
        if let Err(e) = self.write(level, args) {
            eprintln!("niri-helper: failed to write log: {e}");
        }
    }

    pub fn info(&mut self, args: fmt::Arguments<'_>) {
        self.log(Level::Info, args);
    }

    pub fn warn(&mut self, args: fmt::Arguments<'_>) {
        self.log(Level::Warn, args);
    }

    pub fn error(&mut self, args: fmt::Arguments<'_>) {
        self.log(Level::Error, args);
    }

    fn rotate(&mut self) -> io::Result<()> {
        let mut rotated = self.path.clone().into_os_string();
        rotated.push(".1");
        fs::rename(&self.path, PathBuf::from(rotated))?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        self.size = 0;
        Ok(())
    }

    fn write(&mut self, level: Level, args: fmt::Arguments<'_>) -> io::Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("{timestamp} {level} {args}\n");
        if self.size > 0 && self.size + line.len() as u64 > MAX_LOG_SIZE {
            self.rotate()?;
        }
        self.file.write_all(line.as_bytes())?;
        self.size += line.len() as u64;
        Ok(())
    }
}

fn log_path() -> PathBuf {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"));
    base.join("niri-helper").join("helper.log")
}

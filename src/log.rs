use std::env;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

struct Inner {
    file: File,
    size: u64,
}

#[derive(Clone)]
pub struct Logger {
    path: PathBuf,
    inner: Arc<Mutex<Inner>>,
}

impl Logger {
    pub fn init() -> io::Result<Self> {
        let path = log_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            inner: Arc::new(Mutex::new(Inner { file, size })),
        })
    }

    pub fn log(&self, level: Level, args: fmt::Arguments<'_>) {
        if let Err(e) = self.write(level, args) {
            eprintln!("niri-helper: failed to write log: {e}");
        }
    }

    pub fn info(&self, args: fmt::Arguments<'_>) {
        self.log(Level::Info, args);
    }

    pub fn warn(&self, args: fmt::Arguments<'_>) {
        self.log(Level::Warn, args);
    }

    pub fn error(&self, args: fmt::Arguments<'_>) {
        self.log(Level::Error, args);
    }

    fn write(&self, level: Level, args: fmt::Arguments<'_>) -> io::Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("{timestamp} {level} {args}\n");
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.size > 0 && inner.size + line.len() as u64 > MAX_LOG_SIZE {
            self.rotate(&mut inner)?;
        }
        inner.file.write_all(line.as_bytes())?;
        inner.size += line.len() as u64;
        Ok(())
    }

    fn rotate(&self, inner: &mut Inner) -> io::Result<()> {
        let mut rotated = self.path.clone().into_os_string();
        rotated.push(".1");
        fs::rename(&self.path, PathBuf::from(rotated))?;
        inner.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        inner.size = 0;
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

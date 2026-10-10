use std::fs;
use std::path::PathBuf;
use std::sync::MutexGuard;

pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("niri-helper-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn set_env(key: &str, value: impl AsRef<std::path::Path>) {
    unsafe { std::env::set_var(key, value.as_ref()) }
}

pub fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) }
}

mod common;

use std::fs;

use common::{lock, remove_env, set_env, temp_dir};
use niri_helper::config::{Config, ConfigData};

fn write_config(dir: &std::path::Path, contents: &str) {
    let config_dir = dir.join("niri-helper");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), contents).unwrap();
}

#[test]
fn loads_and_parses_config() {
    let _guard = lock();
    let dir = temp_dir("parse");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");
    write_config(
        &dir,
        "[pin]\napp_id = [\"firefox\"]\n\n[scratchpad]\nworkspace = \"side\"\n\n[scratchpad.applications.mpv]\napp_id = \"^mpv$\"\nexecutable = \"mpv --fullscreen\"\n",
    );

    let config = Config::load().unwrap();
    assert_eq!(config.pin.app_id, vec!["firefox"]);
    assert_eq!(config.scratchpad.workspace, "side");
    assert_eq!(config.scratchpad.applications.len(), 1);
    let mpv = &config.scratchpad.applications["mpv"];
    assert_eq!(mpv.app_id, "^mpv$");
    assert_eq!(mpv.executable, "mpv --fullscreen");
    assert!(config.scratchpad_patterns().unwrap()[0].is_match("mpv"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn defaults_when_config_missing() {
    let _guard = lock();
    let dir = temp_dir("missing");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");

    let config = Config::load().unwrap();
    assert!(config.pin.app_id.is_empty());
    assert_eq!(config.scratchpad.workspace, "scratch");
    assert!(config.scratchpad.applications.is_empty());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn errors_on_invalid_toml() {
    let _guard = lock();
    let dir = temp_dir("invalid");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");
    write_config(&dir, "[pin\napp_id = [\n");

    let err = Config::load().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("config.toml"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn falls_back_to_home_config_when_xdg_unset_or_relative() {
    let _guard = lock();
    let dir = temp_dir("fallback");
    set_env("HOME", &dir);
    set_env("XDG_CONFIG_HOME", "relative/path");

    let config_dir = dir.join(".config").join("niri-helper");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), "[pin]\napp_id = [\"a\"]\n").unwrap();

    let config = Config::load().unwrap();
    assert_eq!(config.pin.app_id, vec!["a"]);

    remove_env("XDG_CONFIG_HOME");
    let config = Config::load().unwrap();
    assert_eq!(config.pin.app_id, vec!["a"]);

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn app_id_patterns_compile_and_reject_invalid_regex() {
    let _guard = lock();
    let dir = temp_dir("patterns");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");
    write_config(&dir, "[pin]\napp_id = [\"^firefox$\", \"mpv\"]\n");

    let config = Config::load().unwrap();
    let patterns = config.app_id_patterns().unwrap();
    assert_eq!(patterns.len(), 2);
    assert!(patterns[0].is_match("firefox"));
    assert!(!patterns[0].is_match("firefox-beta"));

    write_config(&dir, "[pin]\napp_id = [\"(\"]\n");
    let config = Config::load().unwrap();
    assert!(config.app_id_patterns().is_err());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scratchpad_workspace_defaults_to_scratch() {
    let _guard = lock();
    let dir = temp_dir("scratch-default");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");

    write_config(
        &dir,
        "[scratchpad]\n\n[scratchpad.applications.mpv]\napp_id = \"mpv\"\nexecutable = \"mpv\"\n",
    );
    let config = Config::load().unwrap();
    assert_eq!(config.scratchpad.workspace, "scratch");
    assert_eq!(config.scratchpad.applications.len(), 1);

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scratchpad_applications_require_app_id_and_executable() {
    let _guard = lock();
    let dir = temp_dir("apps-required");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");

    write_config(&dir, "[scratchpad.applications.mpv]\napp_id = \"^mpv$\"\n");
    let err = Config::load().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);

    write_config(
        &dir,
        "[scratchpad.applications.mpv]\nexecutable = \"mpv\"\n",
    );
    let err = Config::load().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scratchpad_patterns_reject_invalid_regex() {
    let _guard = lock();
    let dir = temp_dir("apps-regex");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");

    write_config(
        &dir,
        "[scratchpad.applications.mpv]\napp_id = \"(\"\nexecutable = \"mpv\"\n",
    );
    let config = Config::load().unwrap();
    assert!(config.scratchpad_patterns().is_err());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn config_data_loads_and_compiles() {
    let _guard = lock();
    let dir = temp_dir("configdata");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");
    write_config(
        &dir,
        "[pin]\napp_id = [\"^firefox$\"]\n\n[scratchpad.applications.mpv]\napp_id = \"^mpv$\"\nexecutable = \"mpv\"\n",
    );

    let data = ConfigData::load().unwrap();
    assert_eq!(data.pin_patterns.len(), 1);
    assert!(data.pin_patterns[0].is_match("firefox"));
    assert_eq!(data.scratchpad_patterns.len(), 1);
    assert!(data.scratchpad_patterns[0].is_match("mpv"));
    assert_eq!(data.config.scratchpad.workspace, "scratch");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn config_data_rejects_invalid_regex() {
    let _guard = lock();
    let dir = temp_dir("configdata-bad");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");
    write_config(&dir, "[pin]\napp_id = [\"(\"]\n");

    let err = ConfigData::load().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("pin.app_id"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn config_fingerprint_tracks_file_changes() {
    let _guard = lock();
    let dir = temp_dir("fingerprint");
    set_env("XDG_CONFIG_HOME", &dir);
    remove_env("HOME");
    write_config(&dir, "[pin]\napp_id = [\"a\"]\n");

    let first = niri_helper::config::config_fingerprint().unwrap();
    let second = niri_helper::config::config_fingerprint().unwrap();
    assert_eq!(first, second);

    write_config(&dir, "[pin]\napp_id = [\"a\", \"b\"]\n");
    let third = niri_helper::config::config_fingerprint().unwrap();
    assert_ne!(first, third);

    fs::remove_dir_all(&dir).unwrap();
}

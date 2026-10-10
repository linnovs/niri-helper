mod common;

use std::fs;

use common::{lock, remove_env, set_env, temp_dir};
use niri_helper::config::Config;

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
        "[pin]\napp_id = [\"firefox\"]\n\n[scratchpad]\nworkspace = \"side\"\napp_id = [\"^mpv$\"]\n",
    );

    let config = Config::load().unwrap();
    assert_eq!(config.pin.app_id, vec!["firefox"]);
    assert_eq!(config.scratchpad.workspace, "side");
    assert_eq!(config.scratchpad.app_id, vec!["^mpv$"]);

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
    assert!(config.scratchpad.app_id.is_empty());

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

    write_config(&dir, "[scratchpad]\napp_id = [\"mpv\"]\n");
    let config = Config::load().unwrap();
    assert_eq!(config.scratchpad.workspace, "scratch");
    assert_eq!(config.scratchpad.app_id, vec!["mpv"]);

    fs::remove_dir_all(&dir).unwrap();
}

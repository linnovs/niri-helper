mod common;

use std::fs;

use common::{lock, remove_env, set_env, temp_dir};
use niri_helper::log::Logger;

const MAX_LOG_SIZE: u64 = 1024 * 1024;

#[test]
fn writes_lines_with_timestamp_and_level() {
    let _guard = lock();
    let dir = temp_dir("log-basic");
    set_env("XDG_CACHE_HOME", &dir);
    remove_env("HOME");

    let mut logger = Logger::init().unwrap();
    logger.info(format_args!("hello {}", "world"));
    logger.warn(format_args!("oops"));

    let contents = fs::read_to_string(dir.join("niri-helper").join("helper.log")).unwrap();
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 2);

    let pattern =
        regex::Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} (INFO|WARN|ERROR) .+$").unwrap();
    assert!(pattern.is_match(lines[0]), "line: {}", lines[0]);
    assert!(pattern.is_match(lines[1]), "line: {}", lines[1]);
    assert!(lines[0].ends_with("hello world"));
    assert!(lines[1].ends_with("oops"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rotates_to_gover1_when_capacity_reached() {
    let _guard = lock();
    let dir = temp_dir("log-rotate");
    set_env("XDG_CACHE_HOME", &dir);
    remove_env("HOME");

    let log_dir = dir.join("niri-helper");
    fs::create_dir_all(&log_dir).unwrap();
    let path = log_dir.join("helper.log");
    let fill = MAX_LOG_SIZE - 1;
    fs::write(&path, vec![b'x'; fill as usize]).unwrap();

    let mut logger = Logger::init().unwrap();
    logger.error(format_args!("this line forces a rotation"));

    let rotated = log_dir.join("helper.log.1");
    assert_eq!(fs::metadata(&rotated).unwrap().len(), fill);
    assert!(fs::metadata(&path).unwrap().len() < MAX_LOG_SIZE);
    let current = fs::read_to_string(&path).unwrap();
    assert!(
        current
            .trim_end()
            .ends_with("ERROR this line forces a rotation")
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn falls_back_to_home_cache() {
    let _guard = lock();
    let dir = temp_dir("log-fallback");
    set_env("HOME", &dir);
    set_env("XDG_CACHE_HOME", "relative/path");

    let mut logger = Logger::init().unwrap();
    logger.info(format_args!("hello"));

    let path = dir.join(".cache").join("niri-helper").join("helper.log");
    assert!(path.exists());
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .trim_end()
            .ends_with("hello")
    );

    fs::remove_dir_all(&dir).unwrap();
}

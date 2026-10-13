use niri_helper::scratch::{matches, refocus_target};
use niri_ipc::{Timestamp, Window, WindowLayout};
use regex::Regex;
use std::time::Duration;

fn window(app_id: Option<&str>) -> Window {
    Window {
        id: 1,
        title: None,
        app_id: app_id.map(str::to_string),
        pid: None,
        workspace_id: Some(1),
        is_focused: false,
        is_floating: true,
        is_urgent: false,
        layout: WindowLayout {
            pos_in_scrolling_layout: None,
            tile_size: (0.0, 0.0),
            window_size: (0, 0),
            tile_pos_in_workspace_view: None,
            window_offset_in_tile: (0.0, 0.0),
        },
        focus_timestamp: None,
    }
}

fn test_window(
    id: u64,
    app_id: Option<&str>,
    ws: Option<u64>,
    focused: bool,
    floating: bool,
    focus_secs: Option<(u64, u32)>,
) -> Window {
    let mut w = window(app_id);
    w.id = id;
    w.workspace_id = ws;
    w.is_focused = focused;
    w.is_floating = floating;
    w.focus_timestamp = focus_secs.map(|(secs, nanos)| Timestamp::from(Duration::new(secs, nanos)));
    w
}

fn patterns(list: &[&str]) -> Vec<Regex> {
    list.iter().map(|p| Regex::new(p).unwrap()).collect()
}

#[test]
fn matches_any_configured_regex() {
    let pats = patterns(&["^org\\.mozilla\\.Firefox$", "mpv"]);
    assert!(matches(&pats, &window(Some("org.mozilla.Firefox"))));
    assert!(matches(&pats, &window(Some("mpv"))));
    assert!(!matches(&pats, &window(Some("org.mozilla.FirefoxBeta"))));
    assert!(!matches(&pats, &window(Some("vlc"))));
}

#[test]
fn matches_is_case_sensitive() {
    let pats = patterns(&["firefox"]);
    assert!(matches(&pats, &window(Some("firefox"))));
    assert!(!matches(&pats, &window(Some("Firefox"))));
}

#[test]
fn no_match_when_app_id_missing_or_patterns_empty() {
    assert!(!matches(&patterns(&["firefox"]), &window(None)));

    let empty = patterns(&[]);
    assert!(!matches(&empty, &window(Some("firefox"))));
}

#[test]
fn refocus_targets_most_recently_focused_tiled_window_when_focused_is_pinned_float() {
    let pin = patterns(&["pinned-app"]);
    let windows = vec![
        test_window(1, Some("pinned-app"), Some(1), true, true, None),
        test_window(2, Some("tiled-a"), Some(1), false, false, Some((10, 0))),
        test_window(3, Some("tiled-b"), Some(1), false, false, Some((20, 0))),
    ];
    assert_eq!(refocus_target(&windows, 1, &pin), Some(3));
}

#[test]
fn refocus_returns_none_when_focus_is_already_tiled() {
    let pin = patterns(&["pinned-app"]);
    let windows = vec![
        test_window(1, Some("tiled-a"), Some(1), true, false, Some((10, 0))),
        test_window(2, Some("tiled-b"), Some(1), false, false, Some((20, 0))),
    ];
    assert_eq!(refocus_target(&windows, 1, &pin), None);
}

#[test]
fn refocus_returns_none_when_focused_float_is_not_pinned() {
    let pin = patterns(&["pinned-app"]);
    let windows = vec![
        test_window(1, Some("other-float"), Some(1), true, true, None),
        test_window(2, Some("tiled-a"), Some(1), false, false, Some((10, 0))),
    ];
    assert_eq!(refocus_target(&windows, 1, &pin), None);
}

#[test]
fn refocus_returns_none_when_no_tiled_window_on_workspace() {
    let pin = patterns(&["pinned-app"]);
    let windows = vec![
        test_window(1, Some("pinned-app"), Some(1), true, true, None),
        test_window(
            2,
            Some("tiled-elsewhere"),
            Some(2),
            false,
            false,
            Some((30, 0)),
        ),
    ];
    assert_eq!(refocus_target(&windows, 1, &pin), None);
}

#[test]
fn refocus_prefers_timestamped_over_untimestamped_tiled_windows() {
    let pin = patterns(&["pinned-app"]);
    let windows = vec![
        test_window(1, Some("pinned-app"), Some(1), true, true, None),
        test_window(2, Some("tiled-a"), Some(1), false, false, None),
        test_window(3, Some("tiled-b"), Some(1), false, false, Some((5, 0))),
    ];
    assert_eq!(refocus_target(&windows, 1, &pin), Some(3));
}

use niri_helper::scratch::matches;
use niri_ipc::{Window, WindowLayout};
use regex::Regex;

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

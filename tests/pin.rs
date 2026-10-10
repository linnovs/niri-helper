use niri_helper::pin::pinned_window_ids;
use niri_ipc::state::EventStreamState;
use niri_ipc::{Window, WindowLayout, Workspace};
use regex::Regex;

fn workspace(id: u64, output: &str) -> Workspace {
    Workspace {
        id,
        idx: 1,
        name: None,
        output: Some(output.to_string()),
        is_urgent: false,
        is_active: false,
        is_focused: false,
        active_window_id: None,
    }
}

fn window(id: u64, app_id: &str, ws: u64, floating: bool) -> Window {
    Window {
        id,
        title: None,
        app_id: Some(app_id.to_string()),
        pid: None,
        workspace_id: Some(ws),
        is_focused: false,
        is_floating: floating,
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

#[test]
fn includes_floating_matching_windows_on_same_monitor() {
    let patterns = vec![Regex::new("^firefox$").unwrap()];
    let mut state = EventStreamState::default();
    state.workspaces.workspaces.insert(1, workspace(1, "DP-1"));
    state.workspaces.workspaces.insert(2, workspace(2, "DP-1"));
    state
        .windows
        .windows
        .insert(10, window(10, "firefox", 2, true));

    assert_eq!(pinned_window_ids(&state, 1, &patterns), vec![10]);
}

#[test]
fn excludes_non_matching_non_floating_other_monitor_and_already_target() {
    let patterns = vec![Regex::new("^firefox$").unwrap()];
    let mut state = EventStreamState::default();
    state.workspaces.workspaces.insert(1, workspace(1, "DP-1"));
    state.workspaces.workspaces.insert(2, workspace(2, "DP-1"));
    state.workspaces.workspaces.insert(3, workspace(3, "DP-2"));
    state
        .windows
        .windows
        .insert(11, window(11, "msedge", 2, true));
    state
        .windows
        .windows
        .insert(12, window(12, "firefox", 3, true));
    state
        .windows
        .windows
        .insert(13, window(13, "firefox", 2, false));
    state
        .windows
        .windows
        .insert(14, window(14, "firefox", 1, true));

    assert!(pinned_window_ids(&state, 1, &patterns).is_empty());
}

#[test]
fn returns_all_matches_on_the_same_monitor() {
    let patterns = vec![Regex::new("(?i)firefox").unwrap()];
    let mut state = EventStreamState::default();
    state.workspaces.workspaces.insert(1, workspace(1, "DP-1"));
    state.workspaces.workspaces.insert(2, workspace(2, "DP-1"));
    state
        .windows
        .windows
        .insert(10, window(10, "firefox", 2, true));
    state
        .windows
        .windows
        .insert(11, window(11, "Firefox", 2, true));

    let mut result = pinned_window_ids(&state, 1, &patterns);
    result.sort();
    assert_eq!(result, vec![10, 11]);
}

#[test]
fn empty_when_target_workspace_missing_or_no_output() {
    let patterns = vec![Regex::new(".*").unwrap()];
    let mut state = EventStreamState::default();

    assert_eq!(pinned_window_ids(&state, 99, &patterns), Vec::<u64>::new());

    state.workspaces.workspaces.insert(1, workspace(1, "DP-1"));
    let ws_no_output = Workspace {
        id: 2,
        idx: 1,
        name: None,
        output: None,
        is_urgent: false,
        is_active: false,
        is_focused: false,
        active_window_id: None,
    };
    state.workspaces.workspaces.insert(2, ws_no_output);
    state
        .windows
        .windows
        .insert(10, window(10, "firefox", 1, true));

    assert_eq!(pinned_window_ids(&state, 1, &patterns), Vec::<u64>::new());
}

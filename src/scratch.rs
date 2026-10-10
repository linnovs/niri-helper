use std::io;
use std::time::Duration;

use niri_ipc::{Action, Request, Response, Window, WorkspaceReferenceArg};
use regex::Regex;
use tokio::io::BufReader;
use tokio::net::UnixStream;
use tokio::time::{Instant, sleep};

use crate::config::Config;
use crate::ipc::{action, connect, send};

const SPAWN_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

async fn bring(socket: &mut BufReader<UnixStream>, window_id: u64, ws_id: u64) -> io::Result<()> {
    action(
        socket,
        Action::MoveWindowToWorkspace {
            window_id: Some(window_id),
            reference: WorkspaceReferenceArg::Id(ws_id),
            focus: true,
        },
    )
    .await?;
    action(
        socket,
        Action::MoveWindowToFloating {
            id: Some(window_id),
        },
    )
    .await?;
    action(socket, Action::FocusWindow { id: window_id }).await?;
    Ok(())
}

async fn hide(
    socket: &mut BufReader<UnixStream>,
    window_id: u64,
    workspace: &str,
) -> io::Result<()> {
    action(
        socket,
        Action::MoveWindowToWorkspace {
            window_id: Some(window_id),
            reference: WorkspaceReferenceArg::Name(workspace.to_string()),
            focus: false,
        },
    )
    .await?;
    action(
        socket,
        Action::MoveWindowToFloating {
            id: Some(window_id),
        },
    )
    .await?;
    Ok(())
}

#[doc(hidden)]
pub fn matches(patterns: &[Regex], window: &Window) -> bool {
    window
        .app_id
        .as_deref()
        .is_some_and(|a| patterns.iter().any(|p| p.is_match(a)))
}

async fn focused_workspace(socket: &mut BufReader<UnixStream>) -> io::Result<u64> {
    match send(socket, Request::Workspaces).await? {
        Ok(Response::Workspaces(workspaces)) => workspaces
            .into_iter()
            .find(|ws| ws.is_focused)
            .map(|ws| ws.id)
            .ok_or_else(|| io::Error::other("no focused workspace")),
        Ok(other) => Err(io::Error::other(format!("unexpected response: {other:?}"))),
        Err(e) => Err(io::Error::other(e)),
    }
}

async fn request_windows(socket: &mut BufReader<UnixStream>) -> io::Result<Vec<Window>> {
    match send(socket, Request::Windows).await? {
        Ok(Response::Windows(windows)) => Ok(windows),
        Ok(other) => Err(io::Error::other(format!("unexpected response: {other:?}"))),
        Err(e) => Err(io::Error::other(e)),
    }
}

pub async fn run(name: String) -> io::Result<()> {
    let config = Config::load()?;
    let Some(application) = config.scratchpad.applications.get(&name) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown scratchpad application '{name}'"),
        ));
    };
    let pattern = Regex::new(&application.app_id).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid app_id regex for '{name}': {e}"),
        )
    })?;
    let patterns = [pattern];

    let mut socket = connect().await?;
    let workspace = config.scratchpad.workspace.clone();

    let focused_ws = focused_workspace(&mut socket).await?;
    let windows = request_windows(&mut socket).await?;

    if let Some(window) = windows.iter().find(|w| matches(&patterns, w)) {
        if window.workspace_id == Some(focused_ws) {
            hide(&mut socket, window.id, &workspace).await?;
        } else {
            bring(&mut socket, window.id, focused_ws).await?;
        }
        return Ok(());
    }

    let before: Vec<u64> = windows.iter().map(|w| w.id).collect();
    action(
        &mut socket,
        Action::SpawnSh {
            command: application.executable.clone(),
        },
    )
    .await?;

    let deadline = Instant::now() + SPAWN_TIMEOUT;
    loop {
        let windows = request_windows(&mut socket).await?;
        if let Some(window) = windows
            .iter()
            .find(|w| matches(&patterns, w) && !before.contains(&w.id))
        {
            bring(&mut socket, window.id, focused_ws).await?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "timed out waiting for the scratchpad window to open",
            ));
        }
        sleep(POLL_INTERVAL).await;
    }
}

use std::sync::{Arc, RwLock};
use std::time::Duration;

use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Action, Event, Request, Response, Window, WorkspaceReferenceArg};
use regex::Regex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::interval;

use crate::config::{ConfigData, config_fingerprint};
use crate::ipc::{action, connect, send};
use crate::log::Logger;
use crate::pin::move_pinned;

const RELOAD_INTERVAL: Duration = Duration::from_secs(1);

async fn move_new_scratchpad_window(
    actions: &mut BufReader<UnixStream>,
    window: &Window,
    workspace: &str,
    patterns: &[Regex],
    logger: &Logger,
) -> std::io::Result<()> {
    if patterns.is_empty() {
        return Ok(());
    }
    let Some(app_id) = window.app_id.as_deref() else {
        return Ok(());
    };
    if !patterns.iter().any(|p| p.is_match(app_id)) {
        return Ok(());
    }

    action(
        actions,
        Action::MoveWindowToWorkspace {
            window_id: Some(window.id),
            reference: WorkspaceReferenceArg::Name(workspace.to_string()),
            focus: false,
        },
    )
    .await?;
    action(
        actions,
        Action::MoveWindowToFloating {
            id: Some(window.id),
        },
    )
    .await?;
    logger.info(format_args!(
        "moved new scratchpad window {} ({app_id}) to workspace {workspace}",
        window.id
    ));
    Ok(())
}

fn spawn_config_reloader(data: Arc<RwLock<ConfigData>>, logger: Logger) {
    tokio::spawn(async move {
        let mut ticker = interval(RELOAD_INTERVAL);
        ticker.tick().await; // the first tick completes immediately; skip it
        let mut last = config_fingerprint();
        loop {
            ticker.tick().await;
            let now = config_fingerprint();
            if now == last {
                continue;
            }
            last = now;
            match ConfigData::load() {
                Ok(new) => {
                    logger.info(format_args!(
                        "config reloaded: pin.app_id = {:?}, scratchpad.workspace = {}, scratchpad.applications = {:?}",
                        new.config.pin.app_id, new.config.scratchpad.workspace, new.config.scratchpad.applications
                    ));
                    *data.write().unwrap() = new;
                }
                Err(e) => {
                    logger.warn(format_args!(
                        "config reload failed, keeping the previous config: {e}"
                    ));
                }
            }
        }
    });
}

pub async fn run() -> std::io::Result<()> {
    let data = Arc::new(RwLock::new(ConfigData::load()?));
    let logger = Logger::init()?;
    {
        let config = &data.read().unwrap().config;
        logger.info(format_args!("niri-helper daemon started"));
        logger.info(format_args!("config: pin.app_id = {:?}", config.pin.app_id));
        logger.info(format_args!(
            "config: scratchpad.workspace = {}, scratchpad.applications = {:?}",
            config.scratchpad.workspace, config.scratchpad.applications
        ));
    }
    spawn_config_reloader(Arc::clone(&data), logger.clone());

    let mut events = connect().await?;
    let mut actions = connect().await?;

    let reply = send(&mut events, Request::EventStream).await?;
    let Ok(Response::Handled) = &reply else {
        let message = format!("subscribe to EventStream failed: {reply:?}");
        logger.error(format_args!("{message}"));
        return Err(std::io::Error::other(message));
    };

    let mut state = EventStreamState::default();
    let mut line = String::new();
    loop {
        line.clear();
        if events.read_line(&mut line).await? == 0 {
            break;
        }
        let event: Event = serde_json::from_str(&line)?;

        let activated = match &event {
            Event::WorkspaceActivated { id, .. } => Some(*id),
            _ => None,
        };
        let new_window = match &event {
            Event::WindowOpenedOrChanged { window } => {
                (!state.windows.windows.contains_key(&window.id)).then(|| window.clone())
            }
            _ => None,
        };
        state.apply(event);

        if let Some(window) = &new_window {
            let (workspace, patterns) = {
                let data = data.read().unwrap();
                (
                    data.config.scratchpad.workspace.clone(),
                    data.scratchpad_patterns.clone(),
                )
            };
            move_new_scratchpad_window(&mut actions, window, &workspace, &patterns, &logger)
                .await?;
        }

        if let Some(ws_id) = activated {
            let patterns = {
                let data = data.read().unwrap();
                data.pin_patterns.clone()
            };
            let output = state
                .workspaces
                .workspaces
                .get(&ws_id)
                .and_then(|ws| ws.output.clone());
            logger.info(format_args!(
                "workspace {ws_id} activated on output {output:?}"
            ));
            move_pinned(&state, ws_id, &patterns, &mut actions, &logger).await?;
        }
    }

    Ok(())
}

use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Action, Event, Request, Response, Window, WorkspaceReferenceArg};
use regex::Regex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;

use crate::config::Config;
use crate::ipc::{action, connect, send};
use crate::log::Logger;
use crate::pin::move_pinned;

pub async fn run() -> std::io::Result<()> {
    let config = Config::load()?;
    let patterns = config.app_id_patterns().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid pin.app_id regex: {e}"),
        )
    })?;

    let mut logger = Logger::init()?;
    logger.info(format_args!("niri-helper daemon started"));
    logger.info(format_args!("config: pin.app_id = {:?}", config.pin.app_id));

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
        state.apply(event);

        if let Some(ws_id) = activated {
            let output = state
                .workspaces
                .workspaces
                .get(&ws_id)
                .and_then(|ws| ws.output.clone());
            logger.info(format_args!(
                "workspace {ws_id} activated on output {output:?}"
            ));
            move_pinned(&state, ws_id, &patterns, &mut actions, &mut logger).await?;
        }
    }

    Ok(())
}

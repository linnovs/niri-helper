use niri_ipc::state::EventStreamState;
use niri_ipc::{Action, Request, Response, WorkspaceReferenceArg};
use regex::Regex;
use tokio::io::BufReader;
use tokio::net::UnixStream;

use crate::ipc::send;
use crate::log::Logger;

pub async fn move_pinned(
    state: &EventStreamState,
    ws_id: u64,
    patterns: &[Regex],
    actions: &mut BufReader<UnixStream>,
    logger: &mut Logger,
) -> std::io::Result<()> {
    let Some(target) = state.workspaces.workspaces.get(&ws_id) else {
        return Ok(());
    };
    let Some(output) = target.output.as_deref() else {
        return Ok(());
    };

    for window_id in pinned_window_ids(state, ws_id, patterns) {
        let reply = send(
            actions,
            Request::Action(Action::MoveWindowToWorkspace {
                window_id: Some(window_id),
                reference: WorkspaceReferenceArg::Id(ws_id),
                focus: false,
            }),
        )
        .await?;
        if matches!(reply, Ok(Response::Handled)) {
            logger.info(format_args!(
                "moved pinned window {window_id} to workspace {ws_id} on output {output}"
            ));
        } else {
            logger.warn(format_args!(
                "failed to move pinned window {window_id} to workspace {ws_id}: {reply:?}"
            ));
        }
    }

    Ok(())
}

#[doc(hidden)]
pub fn pinned_window_ids(state: &EventStreamState, ws_id: u64, patterns: &[Regex]) -> Vec<u64> {
    let Some(target) = state.workspaces.workspaces.get(&ws_id) else {
        return Vec::new();
    };
    let Some(output) = target.output.as_deref() else {
        return Vec::new();
    };

    state
        .windows
        .windows
        .values()
        .filter(|w| w.is_floating)
        .filter(|w| {
            w.app_id
                .as_deref()
                .is_some_and(|a| patterns.iter().any(|p| p.is_match(a)))
        })
        .filter(|w| {
            w.workspace_id.is_some_and(|id| {
                id != ws_id
                    && state
                        .workspaces
                        .workspaces
                        .get(&id)
                        .is_some_and(|ws| ws.output.as_deref() == Some(output))
            })
        })
        .map(|w| w.id)
        .collect()
}

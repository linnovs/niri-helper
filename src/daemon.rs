use std::sync::{Arc, RwLock};
use std::time::Duration;

use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Action, Event, Request, Response, SizeChange, Window, WorkspaceReferenceArg};
use regex::Regex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::interval;

use crate::config::{CompiledRule, ConfigData, config_fingerprint};
use crate::ipc::{action, connect, send};
use crate::log::Logger;
use crate::pin::move_pinned;

const RELOAD_INTERVAL: Duration = Duration::from_secs(1);

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
                        "config reloaded: pin.app_id = {:?}, scratchpad.workspace = {}, scratchpad.applications = {:?}, rules = {:?}",
                        new.config.pin.app_id, new.config.scratchpad.workspace, new.config.scratchpad.applications, new.config.rules
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

/// A point-in-time copy of the reloadable values the event loop needs.
struct Snapshot {
    workspace: String,
    pin_patterns: Vec<Regex>,
    scratchpad_patterns: Vec<Regex>,
    rule_matchers: Vec<CompiledRule>,
}

struct Daemon {
    config: Arc<RwLock<ConfigData>>,
    logger: Logger,
    actions: BufReader<UnixStream>,
    state: EventStreamState,
}

impl Daemon {
    fn snapshot(&self) -> Snapshot {
        let data = self.config.read().expect("config lock poisoned");
        Snapshot {
            workspace: data.config.scratchpad.workspace.clone(),
            pin_patterns: data.pin_patterns.clone(),
            scratchpad_patterns: data.scratchpad_patterns.clone(),
            rule_matchers: data.rule_matchers.clone(),
        }
    }

    async fn handle_event(&mut self, event: Event) -> std::io::Result<()> {
        let activated = match &event {
            Event::WorkspaceActivated { id, .. } => Some(*id),
            _ => None,
        };
        let (is_new, changed, window) = match &event {
            Event::WindowOpenedOrChanged { window } => {
                let is_new = !self.state.windows.windows.contains_key(&window.id);
                let changed = self
                    .state
                    .windows
                    .windows
                    .get(&window.id)
                    .is_some_and(|old| old.title != window.title || old.app_id != window.app_id);
                (is_new, changed, Some(window.clone()))
            }
            _ => (false, false, None),
        };
        self.state.apply(event);

        if let Some(window) = &window {
            self.on_window(window, is_new, changed).await?;
        }
        if let Some(ws_id) = activated {
            self.on_workspace_activated(ws_id).await?;
        }
        Ok(())
    }

    async fn on_window(
        &mut self,
        window: &Window,
        is_new: bool,
        changed: bool,
    ) -> std::io::Result<()> {
        let snapshot = self.snapshot();

        if (is_new || changed)
            && !window.is_floating
            && let Some(rule) = snapshot
                .rule_matchers
                .iter()
                .find(|r| r.matches(window.app_id.as_deref(), window.title.as_deref()))
        {
            self.float_with_rule(window, rule).await?;
        }

        if is_new {
            self.move_to_scratchpad(window, &snapshot.workspace, &snapshot.scratchpad_patterns)
                .await?;
        }

        Ok(())
    }

    async fn float_with_rule(
        &mut self,
        window: &Window,
        rule: &CompiledRule,
    ) -> std::io::Result<()> {
        action(
            &mut self.actions,
            Action::MoveWindowToFloating {
                id: Some(window.id),
            },
        )
        .await?;
        if let Some(size) = rule.size() {
            action(
                &mut self.actions,
                Action::SetWindowWidth {
                    id: Some(window.id),
                    change: SizeChange::SetFixed(size.width),
                },
            )
            .await?;
            action(
                &mut self.actions,
                Action::SetWindowHeight {
                    id: Some(window.id),
                    change: SizeChange::SetFixed(size.height),
                },
            )
            .await?;
        }
        if let Some(position) = rule.position() {
            let (x, y) = position.changes();
            action(
                &mut self.actions,
                Action::MoveFloatingWindow {
                    id: Some(window.id),
                    x,
                    y,
                },
            )
            .await?;
        }
        self.logger
            .info(format_args!("floated window {} matching a rule", window.id));
        Ok(())
    }

    async fn move_to_scratchpad(
        &mut self,
        window: &Window,
        workspace: &str,
        patterns: &[Regex],
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
            &mut self.actions,
            Action::MoveWindowToWorkspace {
                window_id: Some(window.id),
                reference: WorkspaceReferenceArg::Name(workspace.to_string()),
                focus: false,
            },
        )
        .await?;
        action(
            &mut self.actions,
            Action::MoveWindowToTiling {
                id: Some(window.id),
            },
        )
        .await?;
        self.logger.info(format_args!(
            "moved new scratchpad window {} ({app_id}) to workspace {workspace}",
            window.id
        ));
        Ok(())
    }

    async fn on_workspace_activated(&mut self, ws_id: u64) -> std::io::Result<()> {
        let snapshot = self.snapshot();
        let output = self
            .state
            .workspaces
            .workspaces
            .get(&ws_id)
            .and_then(|ws| ws.output.clone());
        self.logger.info(format_args!(
            "workspace {ws_id} activated on output {output:?}"
        ));
        move_pinned(
            &self.state,
            ws_id,
            &snapshot.pin_patterns,
            &mut self.actions,
            &self.logger,
        )
        .await?;
        Ok(())
    }
}

pub async fn run() -> std::io::Result<()> {
    let config = Arc::new(RwLock::new(ConfigData::load()?));
    let logger = Logger::init()?;
    {
        let config = &config.read().unwrap().config;
        logger.info(format_args!("niri-helper daemon started"));
        logger.info(format_args!("config: pin.app_id = {:?}", config.pin.app_id));
        logger.info(format_args!(
            "config: scratchpad.workspace = {}, scratchpad.applications = {:?}",
            config.scratchpad.workspace, config.scratchpad.applications
        ));
        logger.info(format_args!("config: rules = {:?}", config.rules));
    }
    spawn_config_reloader(Arc::clone(&config), logger.clone());

    let mut events = connect().await?;
    let actions = connect().await?;

    let reply = send(&mut events, Request::EventStream).await?;
    let Ok(Response::Handled) = &reply else {
        let message = format!("subscribe to EventStream failed: {reply:?}");
        logger.error(format_args!("{message}"));
        return Err(std::io::Error::other(message));
    };

    let mut daemon = Daemon {
        config,
        logger,
        actions,
        state: EventStreamState::default(),
    };
    let mut line = String::new();
    loop {
        line.clear();
        if events.read_line(&mut line).await? == 0 {
            break;
        }
        let event: Event = serde_json::from_str(&line)?;
        println!("{event:?}");
        daemon.handle_event(event).await?;
    }

    Ok(())
}

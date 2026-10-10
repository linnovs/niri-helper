# AGENTS.md

Rust (edition 2024) daemon/CLI helper that drives the niri Wayland compositor over its IPC socket. Lib + bin split: `src/lib.rs` holds all logic and subcommand dispatch; `src/main.rs` is a 4-line `#[tokio::main]` wrapper calling `niri_helper::run()`. `Cargo.toml` declares the bin explicitly.

## Commands
- `cargo build`
- `cargo test` — integration tests only, under `tests/` (no inline `#[cfg(test)]` in `src/`)
- `cargo clippy --all-targets` (zero-warning policy)
- `cargo fmt` then `cargo fmt --check`
- Smoke-test dispatch without niri: `cargo run -- --help`

## Environment gotchas
- This sandbox has no default toolchain and `cargo`/`rustup` aren't on PATH. Prefix every cargo command with:
  `export RUSTUP_HOME=$HOME/.rustup CARGO_HOME=$HOME/.local/share/cargo PATH=$HOME/.local/share/cargo/bin:$PATH`
  and if clippy/rustfmt report missing, `rustup component add clippy rustfmt` (the `--profile minimal` toolchain lacks them).
- Everything requires a live niri session: `NIRI_SOCKET` unset fails with a "NIRI_SOCKET is not set" error.

## Config & logging
- Config: `$XDG_CONFIG_HOME/niri-helper/config.toml` (relative XDG falls back to `~/.config`). Keys:
  - `[pin] app_id = ["<regex>", ...]`
  - `[[rules]] app_id = "<regex>"` + `title = "<regex>"` — daemon floats newly opened matching windows (AND within a rule); optional `size = { width, height }` (logical px) and `position = { x, y }` (px or `"NN%"` proportion) applied on float
  - `[scratchpad] workspace = "scratch"` (default), plus `[scratchpad.applications.<name>] app_id = "..."` and `executable = "..."` — dotted keys deserialize into `HashMap<String, Application>`; `app_id`/`executable` required.
- Log: `$XDG_CACHE_HOME/niri-helper/helper.log` (fallback `~/.cache`), rotated to `helper.log.1` at 1 MiB.
- Missing config file => defaults; invalid TOML or regex => fatal (nonzero exit).

## Behavior (not obvious from filenames)
- Subcommands: `daemon` (default, event-stream loop in `src/daemon.rs`) and `scratch <name>` (`src/scratch.rs` looks up `scratchpad.applications`).
- `daemon` moves **floating** windows matching `pin.app_id` to the newly active workspace on their monitor (`focus: false`), and moves **newly opened** windows matching any scratchpad `app_id` to `scratchpad.workspace` (floating, no focus).
- De-facto "new window" is `Event::WindowOpenedOrChanged` for an id absent from `EventStreamState` — that event also fires on updates, so gate on the id.
- `scratch` toggles: window on current workspace => hide to scratchpad workspace; elsewhere => bring (move + float + focus); none => spawn `executable` via `Action::SpawnSh` and poll for the new window.

## niri IPC gotchas
- After `Request::EventStream`, niri stops reading further requests on that socket — actions must use a **second** connection. The daemon opens two `ipc::connect()` sockets (`events` + `actions`).
- `ipc::action()` converts niri `Err(message)` replies to `io::Error`; `ipc::send()` returns the raw `Reply`.
- Move by `WorkspaceReferenceArg::Id` for the focused workspace, `Name` for the scratchpad workspace (named workspaces are created on demand by niri).

## Tests
- Integration tests import the lib crate: `use niri_helper::...` (crate name `niri_helper`, not `niri-helper`). They can only reach `pub` items — `pin::pinned_window_ids` and `scratch::matches` are `pub #[doc(hidden)]` solely for this.
- Env-touching tests must hold `tests/common::lock()` (`ENV_LOCK`): env is process-global and tests run in parallel. Separate test binaries run in separate processes, so no cross-file locking.
- Edition 2024 makes `std::env::set_var`/`remove_var` `unsafe` — the helpers in `tests/common/mod.rs` already wrap them.
- No live niri in CI/sandbox, so async IPC paths (`daemon::run`, `scratch::run`) are untested; tests cover pure logic only.
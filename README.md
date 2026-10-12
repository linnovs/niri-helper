# niri-helper

A personal CLI helper for the [niri](https://github.com/niri-wm/niri) scrollable-tiling
Wayland compositor. It talks to niri over IPC using the
[`niri_ipc`](https://crates.io/crates/niri_ipc) crate. A long-running `daemon`
reacts to compositor events, and a one-shot `scratch` command manages scratchpad
applications.

## Features

- **Pin (sticky windows)** — floating windows matching `pin.app_id` are moved to
  the newly active workspace on their monitor, so they stay visible as you
  switch workspaces. Focus is never stolen.
- **Float rules** — `rules` is an array of `{ app_id, title }` regex pairs; the
  daemon floats newly opened windows whose `app_id` **and** `title` both match.
- **Scratchpad** — applications declared under `scratchpad.applications` live on
  the `scratchpad.workspace` workspace, **tiled** while hidden. `scratch <name>`
  brings one to the current workspace (moves it, then floats it, and focuses it),
  or hides it back (moving and tiling it again).
- **Logging** — daemon logs to `$XDG_CACHE_HOME/niri-helper/helper.log`, rotated
  to `helper.log.1` when it exceeds 1 MiB.

## Requirements

- A running niri session (exposes `NIRI_SOCKET`).
- A Rust toolchain (stable).

## Build

```sh
cargo build --release
```

## Usage

```sh
# Run the event-stream daemon (also the default subcommand)
niri-helper daemon

# Toggle a scratchpad application by its configured name
niri-helper scratch mpv
```

Run `niri-helper --help` for a summary of the subcommands.

## Configuration

Config lives in `$XDG_CONFIG_HOME/niri-helper/config.toml` (relative
`XDG_CONFIG_HOME` values fall back to `~/.config`). If the file is missing,
defaults are used.

```toml
[pin]
app_id = ["^org\\.mozilla\\.Firefox$"]

[[rules]]
app_id = "^mpv$"                             # regex matching the window's app_id
title = ".*"                                 # regex matching the window's title
size = { width = 800, height = 600 }         # optional, logical pixels
position = { x = 100, y = "50%" }            # optional; "50%" = working-area proportion

[scratchpad]
workspace = "scratch"            # optional, defaults to "scratch"

[scratchpad.applications.mpv]
app_id = "^mpv$"                 # regex matching the window
executable = "mpv --fullscreen"  # spawned when no mpv window exists

[scratchpad.applications.feh]
app_id = "^feh$"
executable = "feh"
```

- `pin.app_id` and `scratchpad.applications.*.app_id` are regexes matched against
  a window's `app_id`.
- `rules` is an array of `{ app_id, title }` regex pairs; the daemon floats a
  newly opened window when both regexes match (AND within a rule, OR across
  rules). Optional `size` and `position` are applied when the window is floated:
  `size` is absolute logical pixels; `position` accepts absolute pixels or
  `"NN%"` proportions of the working area.
- `scratch <name>` looks up the application in `scratchpad.applications`; errors
  out if the name is unknown.
- Invalid TOML or a bad regex is a fatal config error.

## Start with Niri

Add to your `niri` configuration (`~/.config/niri/config.kdl`):

```kdl
spawn-at-startup "niri-helper"
binds Cmd+D exec "niri-helper scratch mpv"
```

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.
# niri-helper

A personal long-running CLI helper for the [niri](https://github.com/niri-wm/niri)
scrollable-tiling Wayland compositor. It connects to niri over IPC using the
[`niri_ipc`](https://crates.io/crates/niri_ipc) crate and runs continuously,
reacting to the compositor as you work.

## Features

- **Sticky window** — keeps a window pinned/visible as you navigate niri. (Currently the only helper.)

## How It Works

The binary is a daemon. On startup it connects to niri's IPC socket with
`niri_ipc::Socket::connect()`, subscribes to compositor events, and issues
actions (via `niri_ipc::Request`) in response. It is meant to keep running for
the whole session.

## Requirements

- A running niri session (exposes `NIRI_SOCKET`).
- A Rust toolchain (stable).

## Build

```sh
cargo build --release
```

## Run

```sh
cargo run --release
# or the built binary
./target/release/niri-helper
```

### Start with Niri

Add to your `niri` configuration (`~/.config/niri/config.kdl`):

```kdl
spawn-at-startup "niri-helper"
```

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

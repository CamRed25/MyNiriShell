# DE — Niri Shell

A unified GTK4 shell for the [Niri](https://github.com/YaLTeR/niri) Wayland compositor. Combines a panel, dock, and launcher into a single process that connects to Niri via its IPC socket for live compositor state.

## Project Layout

```
niri-shell/   — main Rust crate (the shell binary)
mockup/       — HTML mockup of the desktop UI
```

## niri-shell

### Components

| Component | Backend | UI |
|---|---|---|
| Panel | `panel_backend.rs` | `panel_ui.rs` |
| Dock | `dock_backend.rs` | `dock_ui.rs` |
| Launcher | `launcher_backend.rs` | `launcher_ui.rs` |

- **Panel** — status bar with workspace indicators, media playback, system stats (CPU/memory/volume), weather, and notifications
- **Dock** — shows active windows from the compositor; highlights the focused window
- **Launcher** — app search with fuzzy matching, match highlighting, and a built-in calculator

### How it works

On startup the shell sends a `SIGTERM` to any prior running instance of itself (Niri auto-restarts `spawn-at-startup` processes, so this prevents briefly running two shells at once). It then:

1. Initialises GTK4 and a sysinfo sampler
2. Connects to the Niri IPC socket (`$NIRI_SOCKET`) and subscribes to the event stream
3. Spawns a background thread that fetches weather every 10 minutes (via HTTP)
4. Builds all three GTK4 layer-shell windows (panel, dock, launcher)

Niri IPC events (workspace changes, window open/close/focus) are forwarded to the GTK main thread every 50 ms and applied to `ShellState`, which drives reactive UI updates.

### Source structure

```
src/
  main.rs              — entry point + prior-instance replacement
  shell.rs             — GTK4 Application setup, IPC wiring, weather polling
  state.rs             — ShellState: applies niri IPC events to panel/dock state
  panel_backend.rs     — pure Rust panel data model and update methods
  panel_ui.rs          — GTK4 panel window (layer shell)
  dock_backend.rs      — pure Rust dock data model
  dock_ui.rs           — GTK4 dock window (layer shell)
  launcher_backend.rs  — app entry scanning, fuzzy search, calculator
  launcher_ui.rs       — GTK4 launcher window
  ipc/                 — niri IPC client (Unix socket, JSON event stream)
  media.rs             — media session integration (zbus / MPRIS)
  sysinfo.rs           — CPU, memory, volume sampling
  weather.rs           — weather fetch and snapshot types
  error.rs             — top-level error types
```

### Key dependencies

- `gtk4` + `gtk4-layer-shell` — UI and Wayland layer surface
- `zbus` — D-Bus / MPRIS media session integration
- `ureq` — HTTP client for weather fetching
- `serde` / `serde_json` — IPC message serialisation
- `thiserror` — error types

### Build & run

```sh
cd niri-shell
cargo build
cargo run
```

Requires GTK4 and `gtk4-layer-shell` system libraries. The shell must be launched inside a running Niri session (it reads `$NIRI_SOCKET`); if the socket is unavailable it starts in a degraded mode without live compositor state.

### Testing

```sh
cd niri-shell
cargo test
```

Backends (`panel_backend`, `dock_backend`, `launcher_backend`) contain pure Rust logic with no GTK4 dependency and are fully unit-testable.

# niri Desktop Environment

Wayland desktop shell for the niri compositor, written entirely in Rust + GTK4. A single binary (`niri-shell`) that integrates three components:

- **Panel** — top status bar (workspaces, media, sysinfo, weather, clock, quick settings, notifications)
- **Dock** — vertical app bar (active Niri windows + pinned apps with drag-and-drop reorder)
- **Launcher** — app search dialog (fuzzy `.desktop` search, built-in calculator, keyboard nav)

All three are compiled into one crate at `niri-shell/`. **Do not split into separate crates.**

## Source Layout

```
niri-shell/src/
├── main.rs                       # Entry point; prior-instance replacement (SIGTERM)
├── shell.rs                      # GTK Application setup, IPC wiring, weather polling
├── shell/                        # Shell submodules
│   ├── config.rs                 # Shell configuration loading
│   ├── input.rs                  # Input event handling
│   ├── launcher.rs               # Launcher integration
│   ├── monitor.rs                # Monitor/output management
│   ├── panel.rs                  # Panel submodule logic
│   ├── protocol.rs               # Protocol definitions
│   └── window_manager.rs         # Window management logic
├── state.rs                      # ShellState: applies Niri IPC events to panel/dock
├── error.rs                      # Top-level error types (ShellError, IpcError)
│
├── panel_backend.rs              # Panel data model (pure Rust, zero GTK)
├── panel_ui.rs                   # GTK4 panel layer-shell window
├── dock_backend.rs               # Dock data model (pinned + active items, reorder)
├── dock_ui.rs                    # GTK4 dock layer-shell window
├── launcher_backend.rs           # App loading, fuzzy search, calculator eval
├── launcher_ui.rs                # GTK4 launcher dialog (centered, keyboard-driven)
├── quick_settings_backend.rs     # Quick settings data model (WiFi/BT/VPN/NightLight tiles)
├── quick_settings_ui.rs          # GTK4 quick settings layer-shell overlay (top-right)
├── notification_daemon.rs        # D-Bus org.freedesktop.Notifications server
├── notification_ui.rs            # GTK4 toast popups + notification centre
├── polkit_agent.rs               # org.freedesktop.PolicyKit1.AuthenticationAgent via zbus
├── polkit_ui.rs                  # GTK4 modal password dialog for privilege escalation
├── power_backend.rs              # Power/session management (lock, logout, suspend)
├── power_ui.rs                   # GTK4 power menu UI
│
├── ipc/mod.rs                    # Unix socket client, Niri event stream, action sender
├── ipc/types.rs                  # Serde types for Niri IPC (events, actions, requests)
│
├── media.rs                      # MPRIS2 D-Bus polling (zbus) + playback control
├── sysinfo.rs                    # CPU, memory, network, volume (/proc/stat, /proc/meminfo)
├── weather.rs                    # HTTP fetch from wttr.in, JSON parse
├── osd_ui.rs                     # Volume/brightness OSD overlay (in progress)
└── screenshot_ui.rs              # Screenshot mode picker overlay (in progress)
```

## Build & Test

```sh
cd niri-shell
cargo check --quiet
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt
```

## Hard Rules (Enforced on Every Request)

### 1. No Adwaita — ever
Never use `libadwaita`, any `adw::` type, Adwaita CSS classes (`"card"`, `"suggested-action"`, etc.), or `adw::Application`. Use raw `gtk4::` widgets and custom CSS.

### 2. Backend and UI are always separate files
- `*_ui.rs` → GTK4 only. No business logic. Reads types/state from `*_backend.rs`.
- `*_backend.rs` → Pure Rust. Zero `gtk4` / `glib` imports. Fully unit-testable.

### 3. Typed errors with `thiserror`
All public functions return `Result<T, SomethingError>`. No `Box<dyn Error>`. No `.unwrap()` outside tests / `main()`.

### 4. Log, don't print
Use `log::info!`, `log::warn!`, `log::error!`. Never `println!` / `eprintln!` in production paths.

### 5. Dependency minimalism
Check `std` first (`OnceLock`, `mpsc`, `Path`, `str::parse`, etc.). If a crate is already in `Cargo.toml`, use it. Only add a new crate for a real protocol/algorithm that would take >30 lines to get right. **Never add** `once_cell`, `lazy_static`, `itertools`, `dirs`, `strum`, `derive_more`, or `meval`.

### 6. No global mutable state
All shared state is `Rc<RefCell<>>` (GTK main thread) or `Arc<Mutex<>>` (cross-thread). No `static mut`.

## Actual Dependencies (`Cargo.toml`)

```toml
gtk4 = { version = "0.11", features = ["v4_12"] }
glib = "0.22"
gtk4-layer-shell = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
log = "0.4"
env_logger = "0.11"
thiserror = "2"
libc = "0.2"
ureq = "3.3.0"
zbus = "4.1"
zbus_macros = "4.1"
tokio = { version = "1.37.0", features = ["rt-multi-thread", "macros", "sync"] }
futures = "0.3"
```

## Architecture

Niri IPC events arrive on a background thread → forwarded to GTK main thread via `glib::timeout_add_local` (50 ms batch) → `ShellState` applies them → reactive UI widgets update in place.

```
Niri socket ──► IPC thread ──► glib timeout (50ms) ──► ShellState ──► Panel UI
                                                                  └──► Dock UI
Background threads (weather, media, sysinfo) ──────────────────────► Panel UI
                                                                      (via Arc<Mutex>)
```

- **`ShellState`** (`state.rs`) is the single source of truth on the GTK thread. It owns `PanelState` and `DockState`.
- **IPC** (`ipc/`) — Unix socket at `$NIRI_SOCKET`. Reads a JSON event stream; sends `NiriAction` JSON for focus/launch.
- **Media** (`media.rs`) — zbus D-Bus polling on a dedicated thread; playback controls via MPRIS2.
- **Sysinfo** (`sysinfo.rs`) — polls `/proc/stat`, `/proc/meminfo`, and `pactl` on a timer.
- **Weather** (`weather.rs`) — fetches `wttr.in` JSON every 10 minutes on a background thread.
- **Launcher** — shown/hidden via `SIGUSR1`; built-in calculator evaluates arithmetic strings without an external crate.
- **Dock persistence** — pinned apps saved to `~/.config/niri-shell/pins.json` via `serde_json`.

## Visual Design Tokens

All values come from `mockup/` HTML files. Apply via `gtk4::CssProvider`. New UI must match the mockups.

| Property | Value |
|---|---|
| Background | `rgba(15, 15, 25, 0.82)` + `backdrop-filter: blur(24px)` |
| Accent / active | `#7aa2f7` |
| Occupied workspace | `#3d59a1` |
| Empty workspace | `#565f89` |
| UI font | Inter |
| Mono font | JetBrains Mono |
| Corner radius | `10–14px` |

## Rust Conventions

- Edition `2021` (never `2024`)
- Max line length: 100 (`rustfmt`)
- Error types: `<Subsystem>Error` suffix (e.g. `IpcError`, `LauncherError`)
- Prefer borrowing over cloning
- Use iterators over index loops
- No `#[allow(warnings)]` at crate root

## Planned Features (from `niri-shell/TODO.md`)

Work in these areas without changing the above rules:

- **Theme system** — `theme.rs` parses `~/.config/niri-shell/theme.toml` into a typed `ThemeTokens` struct in an `OnceLock`; each `*_ui.rs` interpolates tokens into CSS; no hardcoded hex values remain
- **Wallpaper** — `wallpaper` path token in `theme.toml`; spawn `swaybg -i <path> -m fill` from `shell.rs` on startup
- **Workspace app icons** — replace workspace dots with 16 px app icons grouped by `workspace_id`; use same `.desktop` lookup as dock
- **`org.freedesktop.ScreenSaver` D-Bus stub** — register via zbus; track `Inhibit`/`UnInhibit` calls; sync to Idle Inhibitor QS tile
- **OSD overlay** — `osd_ui.rs`; centred pill, auto-dismiss after ~1.5 s
- **Screenshot switcher** — `screenshot_ui.rs`; region/window/fullscreen via `grim` + `slurp`
- **Calendar popover** — month grid on clock click; no external deps
- **Multi-monitor panel/dock** — panel/dock on all outputs via `gtk4-layer-shell` per-output API; `Vec<PanelWindow>` in `shell.rs`
- **systemd user service** — `niri-shell.service` (`Restart=on-failure`, `After=graphical-session.target`)

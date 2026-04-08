# DE — Niri Shell

A unified GTK4 shell for the [Niri](https://github.com/YaLTeR/niri) Wayland compositor. Combines a panel, dock, launcher, notification daemon, quick settings, polkit agent, power management, and settings GUI into a single process that connects to Niri via its IPC socket for live compositor state.

## Project Layout

```
niri-shell/   — main Rust crate (the shell binary)
mockup/       — HTML mockups used as design reference
```

## niri-shell

### Components

| Component | Backend | UI |
|---|---|---|
| Panel | `panel_backend.rs` | `panel_ui.rs` |
| Dock | `dock_backend.rs` | `dock_ui.rs` |
| Launcher | `launcher_backend.rs` | `launcher_ui.rs` |
| Notification daemon | `notification_daemon.rs` | `notification_ui.rs` |
| Quick settings | `quick_settings_backend.rs` | `quick_settings_ui.rs` |
| Power | `power_backend.rs` | `power_ui.rs` |
| Polkit agent | `polkit_agent.rs` | `polkit_ui.rs` |
| Settings GUI | `settings_backend.rs` | `settings_ui.rs` |
| OSD overlay | — | `osd_ui.rs` |
| Screenshot picker | — | `screenshot_ui.rs` |

- **Panel** — top status bar with workspace indicators, MPRIS2 media playback controls, system stats (CPU/memory/network/volume), weather, clock, notification bell, and quick settings toggle
- **Dock** — horizontal bar anchored to the bottom of the screen showing active Niri windows and pinned apps; highlights the focused window; persists pins to `~/.config/niri-shell/pins.json`; shows unread notification badges; floats above windows with intelligent auto-hide (slides out of view when the mouse leaves, slides back in when the mouse touches the bottom edge)
- **Launcher** — app search with fuzzy matching and frecency scoring, match highlighting, and a built-in calculator; toggled via `SIGUSR1`
- **Notification daemon** — implements `org.freedesktop.Notifications`; displays toast popups anchored top-right with auto-dismiss; persists history to `~/.local/share/niri-shell/notifications.jsonl`; notification centre accessible from the panel bell button
- **Quick settings** — layer-shell overlay anchored top-right; toggle grid for Wi-Fi, Bluetooth, VPN, Night Light, Do Not Disturb, mic mute, power profile, and idle inhibitor; brightness and volume sliders
- **Power** — suspend, reboot, and shutdown via `org.freedesktop.login1`; confirmation dialog opened from the quick settings footer
- **Polkit agent** — implements `org.freedesktop.PolicyKit1.AuthenticationAgent`; shows a GTK4 password dialog when apps request privilege escalation (package managers, drive mounting, etc.)
- **Settings GUI** — visual editor for Niri compositor config: outputs (mode, scale, position, transform), window rules, layer rules, and switch events (lid/tablet-mode bindings); keybindings recorder in progress
- **OSD overlay** — centred pill shown on volume or brightness change, auto-dismisses after ~1.5 s
- **Screenshot picker** — centred overlay with region, window, and fullscreen capture modes via `grim` + `slurp`

  (Please Note, the components not tested fully are, screenshot picker and polkit agent. The rest of the components are tested and working as expected. Just note that the Settings GUI is still a huge work in progress.)
  
### How it works

On startup the shell sends a `SIGTERM` to any prior running instance (Niri auto-restarts `spawn-at-startup` processes, so this prevents briefly running two instances at once). It then:

1. Initialises GTK4 and a sysinfo sampler
2. Registers `org.freedesktop.Notifications` on the session D-Bus and loads persisted notification history
3. Registers `org.freedesktop.PolicyKit1.AuthenticationAgent` on the session D-Bus
4. Connects to the Niri IPC socket (`$NIRI_SOCKET`) and subscribes to the event stream
5. Spawns a background thread that polls MPRIS2 for media session state every 2 s
6. Spawns a background thread that fetches weather every 10 minutes via HTTP
7. Builds all GTK4 layer-shell windows (panel, dock, launcher, quick settings, OSD, screenshot picker)

Niri IPC events (workspace changes, window open/close/focus) are forwarded to the GTK main thread every 50 ms and applied to `ShellState`, which drives reactive UI updates.

### Source structure

```
src/
  main.rs                    — entry point + prior-instance replacement
  shell.rs                   — GTK Application setup, IPC wiring, background threads
  shell/                     — shell subsystem modules
    config.rs                — config loading and validation
    input.rs                 — input handling
    launcher.rs              — launcher integration
    monitor.rs               — multi-monitor detection and layout
    panel.rs                 — panel integration
    protocol.rs              — Wayland protocol helpers
    window_manager.rs        — window creation, focus, movement, layout
  state.rs                   — ShellState: applies Niri IPC events to panel/dock state
  error.rs                   — top-level error types
  panel_backend.rs           — pure Rust panel data model and update methods
  panel_ui.rs                — GTK4 panel window (layer shell, top-anchored)
  dock_backend.rs            — pure Rust dock data model (pinned + active items)
  dock_ui.rs                 — GTK4 dock window (layer shell, bottom-anchored, auto-hide)
  launcher_backend.rs        — app entry scanning, fuzzy + frecency search, calculator
  launcher_ui.rs             — GTK4 launcher dialog (centered, keyboard-driven)
  notification_daemon.rs     — D-Bus org.freedesktop.Notifications server + history store
  notification_ui.rs         — toast popups + notification centre popover
  quick_settings_backend.rs  — quick settings data model and tile state
  quick_settings_ui.rs       — quick settings overlay (tiles, sliders)
  power_backend.rs           — power management via org.freedesktop.login1 (zbus)
  power_ui.rs                — suspend/reboot/shutdown confirmation dialog
  polkit_agent.rs            — D-Bus PolicyKit1 authentication agent
  polkit_ui.rs               — GTK4 password dialog for privilege escalation
  settings_backend.rs        — Niri config read/write (outputs, window rules, layer rules, switch events)
  settings_ui.rs             — GTK4 settings window (tabbed editor)
  osd_ui.rs                  — volume/brightness OSD pill overlay
  screenshot_ui.rs           — screenshot mode picker overlay
  ipc/                       — Niri IPC client (Unix socket, JSON event stream, action sender)
  media.rs                   — MPRIS2 D-Bus polling + playback control (zbus)
  sysinfo.rs                 — CPU, memory, network, volume sampling (/proc/stat, pactl)
  weather.rs                 — weather fetch (wttr.in) and snapshot types
```

### Key dependencies

- `gtk4` + `gtk4-layer-shell` — UI widgets and Wayland layer surfaces
- `zbus` — D-Bus client and server (MPRIS2, notifications, logind, NetworkManager, BlueZ, PolicyKit1)
- `ureq` — HTTP client for weather fetching
- `serde` / `serde_json` — IPC message serialisation and config persistence
- `thiserror` — typed error enums

### Build & run

```sh
cd niri-shell
cargo build
cargo run
```

Requires GTK4 and `gtk4-layer-shell` system libraries. The shell must be launched inside a running Niri session (reads `$NIRI_SOCKET`); if the socket is unavailable it starts in a degraded mode without live compositor state.

To run as a persistent session service:

```sh
systemctl --user enable --now niri-shell.service
```

### Testing

```sh
cd niri-shell
cargo test
cargo clippy -- -D warnings
```

Backends (`panel_backend`, `dock_backend`, `launcher_backend`, `quick_settings_backend`, `power_backend`, `settings_backend`, `notification_daemon`, `polkit_agent`) contain pure Rust logic with no GTK4 dependency and are fully unit-testable.

---

## Planned


Future standalone components under consideration:

- **Niri Lock** — GTK4 screen locker using `gtk4-layer-shell` to cover all outputs; unlock via password or PAM (fingerprint support)
- **Niri Idle** — idle detection daemon using `libinput`/`udev`; exposes a D-Bus API consumed by the quick settings idle tile and `org.freedesktop.ScreenSaver`
- **File Manager** — GTK4, no Adwaita; two-pane layout with bookmarks sidebar, icon/list/column view, Dolphin-style breadcrumb, and trash support
- **Niri XDG portal** — implements the XDG desktop portal interfaces for sandboxed apps, forwarding requests to Niri and the shell (open/save dialogs, screenshots, notifications, etc.)

---

**Please note: all these features are based on Niri 25.11 — any newer versions should be checked for compatibility before implementation.**

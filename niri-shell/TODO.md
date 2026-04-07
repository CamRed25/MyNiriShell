# TODO

## Round 5

- [ ] **Theme system** — `theme.rs` parses `~/.config/niri-shell/theme.toml` into a typed
      `ThemeTokens` struct loaded into an `OnceLock` singleton. Tokens: `accent`, `bg`,
      `bg_border`, `text`, `subtext`, `font_ui`, `font_mono`, `radius`. Each `*_ui.rs` calls
      `theme::get()` and interpolates tokens into its CSS string; no hardcoded hex values remain
      in any UI file after this.
- [ ] **Wallpaper** — add `wallpaper` path token to `theme.toml`; spawn `swaybg -i <path> -m fill`
      from `shell.rs` on startup (kill any previous instance). Changing theme re-spawns it.
- [ ] **Workspace app icons** — replace workspace dots in the panel with 16 px app icons grouped by
      `workspace_id`; icons sourced from the same `.desktop` lookup used by the dock.
- [ ] **`org.freedesktop.ScreenSaver` D-Bus stub** — register the well-known name via zbus; track
      active `Inhibit`/`UnInhibit` calls and sync state to the Idle Inhibitor QS tile so video
      players and presentation tools automatically suppress idle.
- [ ] **Polkit agent** — `polkit_agent.rs` registers
      `org.freedesktop.PolicyKit1.AuthenticationAgent` via zbus on the session bus; launches a
      `polkit_ui.rs` modal password dialog on `InitiateAuthentication`. Without this, GUI apps
      requesting privilege escalation silently fail. (mockup: `mockup/niri_polkit_agent.html`)

---

## Backlog

### System / config

- [ ] **Multi-monitor panel/dock** — panel/dock on all outputs via `gtk4-layer-shell` per-output
      API (`set_monitor`); `Vec<PanelWindow>` in `shell.rs`; each window syncs the same `ShellState`.
- [ ] **`XDG_CURRENT_DESKTOP=niri`** — set in `niri-shell.service` (portal selection, GNOME feature flags).
- [ ] **xdg-desktop-portal** — verify `xdg-desktop-portal-gtk` is running under niri for file
      picker / screenshare in Firefox and Electron apps.

### Future apps

- [ ] **xdg-desktop-portal-niri** — custom portal backend for file picker, screenshare, and
      screenshot interfaces. Needed for Flatpak apps to use native dialogs and screencasting.

- [ ] **File Manager** — GTK4, no Adwaita; two-pane layout (bookmarks sidebar + icon/list/column
      view); Dolphin-style breadcrumb + inline rename; Nautilus-style search + batch ops; async
      directory listing; XDG trash spec via a shared `trash.rs`. GdkPixbuf image thumbnails;
      GStreamer video thumbs optional. (mockup: `mockup/filemanager-mockup.html`)

- [ ] **Niri Settings GUI** — visual KDL editor for `~/.config/niri/config.kdl`: keybindings
      recorder, window rules, output layout, theme token live preview, shell behaviour toggles.
      Typed Rust KDL parser; never writes freeform strings. (mockup: `mockup/niri_settings.html`)
- [ ] **Niri-Idle** — idle detection daemon using `libinput` and `udev` to track user activity; exposes D-Bus
      API for idle time and inhibition. Used by the quick settings idle tile and the proposed
      `org.freedesktop.ScreenSaver` stub.
- [ ] **Niri Lock** — screen locker using `gtk4-layer-shell` to cover all outputs; unlock via password or
      external PAM helper (for fingerprint reader support). (mockup: `mockup/niri_lock.html`)

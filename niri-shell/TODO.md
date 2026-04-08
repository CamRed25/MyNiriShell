# TODO

## Settings GUI — remaining sections

- [ ] **Keybindings** — key recorder, action picker per bind (high effort)

## Backlog

- [ ] **Theme system** — `theme.rs` parses `~/.config/niri-shell/theme.toml` into a typed
      `ThemeTokens` struct (`accent`, `bg`, `bg_border`, `text`, `subtext`, `font_ui`, `font_mono`,
      `radius`). Each `*_ui.rs` interpolates tokens into CSS; no hardcoded hex values remain.
- [ ] **Wallpaper** — `wallpaper` path token in `theme.toml`; spawn `swaybg -i <path> -m fill`
      from `shell.rs` on startup; kill + re-spawn on theme change.
- [ ] **Workspace app icons** — replace workspace dots in the panel with 16 px app icons grouped by
      `workspace_id`; sourced from the same `.desktop` lookup used by the dock.
- [ ] **`org.freedesktop.ScreenSaver` D-Bus stub** — register via zbus; track `Inhibit`/`UnInhibit`;
      sync state to Idle Inhibitor QS tile.
- [ ] **Multi-monitor panel/dock** — panel/dock on all outputs via `gtk4-layer-shell` per-output
       API; `Vec<PanelWindow>` in `shell.rs`.
 [ ] **File Manager** — GTK4, no Adwaita; two-pane layout, breadcrumb nav, async listing, trash, thumbnails. (mockup: `mockup/filemanager-mockup.html`)
 [ ] **Niri XDG portal** — implements the XDG desktop portal interfaces for sandboxed apps, forwarding requests to Niri and the shell (open/save dialogs, screenshots, notifications, etc.)
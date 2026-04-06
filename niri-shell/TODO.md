# TODO

## Quick Settings panel

Mockup: `mockup/niri_quick_settings_clean.html`

Full redesign — layer-shell overlay anchored top-right, matching the mockup layout:

### Header
- Avatar initials + username + hostname
- Lock button → `swaylock` / `hyprlock`

### Connections toggle grid (3-col tiles, accent-coloured when active)
- **Wi-Fi** — NetworkManager D-Bus (`org.freedesktop.NetworkManager`), show SSID when connected
- **Ethernet** — NM D-Bus, read-only connected state
- **VPN** — NM D-Bus active-connection with VPN type
- **Bluetooth** — BlueZ D-Bus (`org.bluez`)
- **Night Light** — `wlr-gamma-control` protocol or `gammastep`/`redshift` subprocess
- **Do Not Disturb** — in-process flag; suppresses notification popups
- **Keyboard layout** — `xkb-switch` subprocess or libxkbcommon query
- **Microphone mute** — `pactl set-source-mute @DEFAULT_SOURCE@ toggle`
- **Power profile** — `powerprofilesctl` (`net.hadess.PowerProfiles` D-Bus), 3 states

### Display & Audio sliders
- **Brightness** — `/sys/class/backlight/*/brightness` or logind D-Bus
- **Volume** — `pactl set-sink-volume @DEFAULT_SINK@` (already wired)

### Footer buttons
- **Settings** → `gtk-launch gnome-control-center`
- **Displays** → `wdisplays`
- **Log out** → `niri msg action quit`

---

## Status bar

- [ ] **Calendar popover on clock click** — small month grid, no external deps.
- [ ] **Workspace app icons** — 16px app icons instead of dots, grouped by `workspace_id`.

## Launcher

- [ ] **Recent files** — read `~/.local/share/recently-used.xbel`, shown when query is empty.
- [ ] **Clipboard history** — `cc` prefix → `cliphist list`; select → `cliphist decode | wl-copy`.

## Dock

- [ ] **Pinned app unread badges** — poll notification count via D-Bus `org.freedesktop.Notifications`.
- [ ] **Recently-closed ghost icons** — on `WindowClosed`, show icon at reduced opacity, fade ~800 ms.

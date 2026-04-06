# TODO

## Quick Settings (removed — not yet wired up)

The quick settings panel (Wi-Fi, Bluetooth, brightness) was removed from the status
bar because the underlying integrations are not implemented:

- **Wi-Fi toggle** — needs NetworkManager D-Bus (`org.freedesktop.NetworkManager`)
- **Bluetooth toggle** — needs BlueZ D-Bus (`org.bluez`)
- **Brightness slider** — needs logind/UPower or direct sysfs writes

Backend types (`QuickSettings`, `update_quick_settings`) are preserved in
`panel_backend.rs` so nothing needs to be re-designed when this is picked up again.
The `build_notifications_popover` pattern can be used as a starting template.

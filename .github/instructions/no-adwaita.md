---
description: 'Prohibit libadwaita and all Adwaita-derived types, CSS classes, and APIs'
applyTo: '**/*.rs, **/Cargo.toml'
---
# No Adwaita

Never use libadwaita, Adwaita widgets, or any Adwaita-derived theming in this project. Applies to Rust code, Cargo.toml, and UI definitions.

## Banned
- The `libadwaita` crate (also listed as `adw` or `libadwaita-rs` in `Cargo.toml`)
- Any `adw::` namespace types (`AdwWindow`, `AdwHeaderBar`, `AdwToolbarView`, `AdwNavigationView`, `AdwClamp`, `AdwToastOverlay`, etc.)
- The `use libadwaita as adw;` import pattern
- `adw::Application` or `adw::ApplicationWindow`
- Adwaita stylesheet classes applied via `add_css_class` (e.g. `"suggested-action"`, `"destructive-action"`, `"card"`, `"boxed-list"`)
- `adw::StyleManager` or any Adwaita color scheme APIs
- GTK UI XML with `<requires lib="libadwaita"/>` or any `AdwXxx` object types

## Use instead
- Raw `gtk4` / `gtk::` types for all widgets (`gtk::Window`, `gtk::HeaderBar`, `gtk::Box`, etc.)
- Custom CSS for theming and styling
- `gtk::Application` (not `adw::Application`)
- Project-specific styling via loaded stylesheets

## Cargo.toml
Never suggest adding `libadwaita` as a dependency. If it already appears, flag it as a violation.

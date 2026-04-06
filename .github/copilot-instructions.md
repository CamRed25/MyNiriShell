# niri Desktop Environment

Wayland desktop environment for the niri compositor, written entirely in Rust + GTK4. Four standalone programs:

- **`niri-shell/`** — core shell / compositor interface
- **`niri-dock/`** — dock bar (active workspace apps + pinned apps with drag-and-drop)
- **`niri-panel/`** — status bar (workspaces, media, clock, weather, network, CPU, memory, volume, quick settings, notifications)
- **`niri-launcher/`** — app launcher (fuzzy .desktop search, built-in calculator, keyboard nav)

Each is an independent Cargo binary crate. Do not pull one crate into another.

## Build & Test

```sh
cargo check --quiet         # from within a crate directory
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt
```

## Hard Rules (Enforced on Every Request)

### 1. No Adwaita — ever
Never use `libadwaita`, any `adw::` type, Adwaita CSS classes (`"card"`, `"suggested-action"`, etc.), or `adw::Application`. Use raw `gtk4::` widgets and custom CSS.

### 2. UI and backend are always separate files
- `*_ui.rs` → GTK4 only. No business logic. Imports types/state from `*_backend.rs`.
- `*_backend.rs` → Pure Rust. Zero `gtk4` / `glib` imports.

### 3. Typed errors with `thiserror`
All public functions return `Result<T, SomethingError>`. No `Box<dyn Error>`. No `.unwrap()` outside tests/`main()`.

### 4. Log, don't print
`log::info!`, `log::warn!`, `log::error!`. Never `println!` / `eprintln!` in production paths.

### 5. Dependency minimalism
Check `std` first (`OnceLock`, `mpsc`, `Path`, `str::parse`, etc.). If it's already in the crate's `Cargo.toml`, use it. Only add a new crate for a real protocol/algorithm that would take >30 lines to get right. Never add `once_cell`, `lazy_static`, `itertools`, `dirs`, `strum`, or `derive_more`.

## Standard Dependencies
```toml
gtk4 = { version = "0.9", features = ["v4_12"] }
glib = "0.20"
log = "0.4"
env_logger = "0.11"
thiserror = "2"
```

## Visual Design Tokens (from `mockup/niri_desktop_mockup.html`)
| Property | Value |
|----------|-------|
| Background | `rgba(15, 15, 25, 0.82)` + `backdrop-filter: blur(24px)` |
| Accent / active | `#7aa2f7` |
| Occupied workspace | `#3d59a1` |
| Empty workspace | `#565f89` |
| UI font | Inter |
| Mono font | JetBrains Mono |
| Corner radius | `14px` |

Apply via `gtk4::CssProvider`. All UI must match `mockup/niri_desktop_mockup.html`.

## Rust Conventions
- Edition `2021` (never `2024`)
- Max line length: 100 (`rustfmt`)
- Error types: `<Subsystem>Error` suffix
- Prefer borrowing over cloning
- Use iterators instead of index loops
- No global mutable state
- No `#[allow(warnings)]` at crate root

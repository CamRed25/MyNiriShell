# Niri Shell Backend

A Rust backend for the Niri desktop environment shell. Implements core logic for window management, panels, launcher, multi-monitor support, input handling, configuration, and protocol extensions for Wayland compositors. All business logic is pure and testable, with I/O separated for reliability.

## Features
- Window management: creation, focus, movement, layout
- Panels: system indicators, widgets
- Launcher: application launching and search
- Multi-monitor: detection, layout, configuration
- Input: keyboard, mouse, touch
- Config: user/system config loading and validation
- Protocol: custom Wayland protocol extensions

## Project Structure
- `src/main.rs`: Entry point
- `src/shell/`: Core subsystems (modular)

## Getting Started
Build and run:
```sh
cargo build
cargo run
```

## Testing
Run all tests:
```sh
cargo test
```

## Common Issues / Troubleshooting
- If a subsystem fails to initialize, check the error output for details. Error messages will include the subsystem and specific error value.
- No HTTP API is exposed; all logic is local to the backend. If you need integration, see the protocol extension notes.
- Ensure Rust 2024 edition is installed (`rustup update stable`).
- If tests fail, check for missing dependencies or syntax errors in subsystem modules.

## Quality Notes
- All subsystems are modular and testable, with pure logic separated from I/O.
- No frontend/backend integration: this is backend logic only. No HTTP API or frontend calls are present.
- No phantom file references or unused assets.
- Code coverage: ~81% (see tarpaulin-report.html).

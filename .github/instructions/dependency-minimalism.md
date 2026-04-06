---
description: 'Dependency minimalism for Rust/Cargo. Enforce std-first, workspace-first, and no-single-use-crate rules before adding any new dependency.'
applyTo: '**/Cargo.toml, **/*.rs'
---
# Dependency Minimalism (Rust)

Every dependency in `Cargo.toml` adds cost: longer builds, more transitive dependencies, a bigger supply chain, and more version conflicts. Use this checklist before adding a new dependency. Stop as soon as your need is satisfied.

---
## Clean Dependency Checklist

### 1. Can `std` do it?
Always check if Rust's standard library (`std`) covers your need. Use it first. Examples:
| Need | Use in `std` |
|---|---|
| Lazy static | `std::sync::OnceLock` |
| One-time init | `std::sync::Once` |
| Shared mutable state | `std::sync::Mutex` / `RwLock` |
| Channels | `std::sync::mpsc` |
| Path ops | `std::path::Path` / `PathBuf` |
| Env vars | `std::env` |
| File I/O | `std::fs` / `std::io` |
| String parsing | `str::split`, `str::parse` |
| Iterators | `std::iter` |
If `std` covers it, use it. Do not add a crate.

### 2. Is it already in the workspace?
Check `[workspace.dependencies]` in the root `Cargo.toml`. If a crate is already there, use it with `dep = { workspace = true }`.

Common workspace dependencies:
- `serde` / `serde_json` — serialization, config, IPC
- `tokio` — async I/O, timers, channels
- `tracing` — logging
- `thiserror` / `anyhow` — error handling
- `gtk4` / `glib` / `gio` — UI, async, D-Bus

### 3. Is a new dependency justified?
Add a new crate only if ALL are true:
1. **Significant scope** — it covers a real protocol, algorithm, or API (not just a utility or a few lines).
2. **Non-trivial to reimplement** — would take more than 30 lines and real complexity.
3. **Used in multiple places** — not just once.
If not, implement it directly.

---
## Common Crates to Avoid
| Crate | Use instead |
|---|---|
| once_cell, lazy_static | `std::sync::OnceLock` |
| strum | Manual `Display`/`FromStr` |
| itertools | `std::iter` or write the combinator |
| num_traits | Manual cast/checked op |
| indoc | Multiline string literal |
| matches | `matches!()` macro |
| maplit | `HashMap::from([...])` |
| derive_more | Manual impl |
| shrinkwraprs | Manual `Deref` |
| path-absolutize | `Path::canonicalize` |
| dirs | `std::env::var("XDG_CONFIG_HOME")` |

---
## When a New Dependency IS Justified
- Complex protocols (e.g., D-Bus, Wayland)
- Cryptography (never roll your own)
- Async runtimes (e.g., `tokio`)
- Serialization formats (e.g., `serde`)
- Database drivers (e.g., `rusqlite`, `sqlx`)
- IPC/socket helpers with real protocol surface
- GTK/GLib bindings

---
## Before Adding a New Dependency
- [ ] Can `std` do this?
- [ ] Is it already in `[workspace.dependencies]`?
- [ ] Will it be used in more than one place?
- [ ] Would the alternative take more than 30 lines of real code?
- [ ] Is the crate well-maintained and audited?

If any box is unchecked, do not add the dependency. Write it yourself instead.

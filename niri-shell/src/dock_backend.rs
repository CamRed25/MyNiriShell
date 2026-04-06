// Dock backend — pure Rust, zero GTK imports.
// Manages pinned apps, active workspace windows, and launch/focus logic.
// Public types are forward-declared for future IPC/D-Bus integration.
#![allow(dead_code)]

use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockError {
    #[error("reorder index out of bounds: from={from}, to={to}, len={len}")]
    ReorderOutOfBounds { from: usize, to: usize, len: usize },

    #[error("failed to launch app '{app}': {source}")]
    LaunchFailed {
        app: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone)]
pub struct DockItem {
    /// App identity used for pinned-vs-active matching and gtk-launch.
    pub id: String,
    pub name: String,
    pub icon: String,
    pub is_active: bool,
    pub is_pinned: bool,
    /// Niri compositor window ID (0 for pinned / unknown).
    pub niri_id: u64,
}

impl DockItem {
    pub fn pinned(id: impl Into<String>, name: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            icon: icon.into(),
            is_active: false,
            is_pinned: true,
            niri_id: 0,
        }
    }

    pub fn active(
        id: impl Into<String>,
        name: impl Into<String>,
        icon: impl Into<String>,
        niri_id: u64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            icon: icon.into(),
            is_active: true,
            is_pinned: false,
            niri_id,
        }
    }
}

pub struct DockState {
    pub pinned: Vec<DockItem>,
    pub active: Vec<DockItem>,
}

impl DockState {
    pub fn new() -> Self {
        Self {
            pinned: vec![
                DockItem::pinned("org.gnome.Nautilus", "Files", "org.gnome.Nautilus"),
                DockItem::pinned("org.gnome.Terminal", "Terminal", "org.gnome.Terminal"),
                DockItem::pinned("firefox", "Firefox", "firefox"),
                DockItem::pinned("org.gnome.Settings", "Settings", "org.gnome.Settings"),
                DockItem::pinned("org.gnome.TextEditor", "Text Editor", "org.gnome.TextEditor"),
                DockItem::pinned("code", "VS Code", "com.visualstudio.code"),
            ],
            active: Vec::new(),
        }
    }

    /// Replace the active-window list with a fresh snapshot from the compositor.
    pub fn set_active_windows(&mut self, windows: Vec<DockItem>) {
        self.active = windows;
    }

    /// Reorder a pinned item from `from` index to `to` index (drag-and-drop support).
    pub fn reorder_pinned(&mut self, from: usize, to: usize) -> Result<(), DockError> {
        let len = self.pinned.len();
        if from >= len || to >= len {
            return Err(DockError::ReorderOutOfBounds { from, to, len });
        }
        let item = self.pinned.remove(from);
        self.pinned.insert(to, item);
        Ok(())
    }

    /// Launch or focus an app. Uses `gtk-launch` for .desktop entries when available,
    /// otherwise falls back to executing the item's id directly.
    pub fn launch(&self, item: &DockItem) -> Result<(), DockError> {
        log::info!("Launching '{}'", item.name);

        // Try gtk-launch first (handles .desktop association and focus-if-running).
        let result = Command::new("gtk-launch").arg(&item.id).spawn();

        match result {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // gtk-launch unavailable — fall back to direct exec.
                Command::new(&item.id).spawn().map(|_| ()).map_err(|source| {
                    DockError::LaunchFailed {
                        app: item.id.clone(),
                        source,
                    }
                })
            }
            Err(source) => Err(DockError::LaunchFailed {
                app: item.id.clone(),
                source,
            }),
        }
    }
}

impl Default for DockState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pinned_apps_are_populated() {
        let state = DockState::new();
        assert!(!state.pinned.is_empty());
        assert!(state.pinned.iter().all(|i| i.is_pinned));
    }

    #[test]
    fn set_active_windows_replaces_list() {
        let mut state = DockState::new();
        let windows = vec![DockItem::active("foo", "Foo", "foo-icon", 99)];
        state.set_active_windows(windows);
        assert_eq!(state.active.len(), 1);
        assert_eq!(state.active[0].id, "foo");
    }

    #[test]
    fn reorder_pinned_moves_item() {
        let mut state = DockState::new();
        let first_id = state.pinned[0].id.clone();
        let second_id = state.pinned[1].id.clone();
        state.reorder_pinned(0, 1).unwrap();
        assert_eq!(state.pinned[0].id, second_id);
        assert_eq!(state.pinned[1].id, first_id);
    }

    #[test]
    fn reorder_pinned_out_of_bounds_returns_error() {
        let mut state = DockState::new();
        let len = state.pinned.len();
        assert!(state.reorder_pinned(0, len).is_err());
    }
}

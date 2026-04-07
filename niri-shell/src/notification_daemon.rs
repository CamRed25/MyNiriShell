// BACKEND ONLY — zero GTK/glib imports.
//
// Registers `org.freedesktop.Notifications` on the session D-Bus and forwards
// every `Notify` / `CloseNotification` call to the GTK main thread via a
// bounded sync channel.
//
// Also provides helpers to persist notifications to a JSONL journal and reload
// them on startup so the notification centre survives a shell restart.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use zbus::interface;

// ── Persistence ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedNotif {
    pub id: u32,
    pub app: String,
    pub summary: String,
    pub body: String,
    pub timestamp: u64,
}

fn journal_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_default();
            std::path::PathBuf::from(home).join(".local/share")
        });
    base.join("niri-shell/notifications.jsonl")
}

const MAX_JOURNAL_ENTRIES: usize = 500;

/// Append a notification to the JSONL journal, keeping at most 500 entries.
pub fn persist_notification(id: u32, app: &str, summary: &str, body: &str, timestamp: u64) {
    let path = journal_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let entry = PersistedNotif {
        id,
        app: app.to_owned(),
        summary: summary.to_owned(),
        body: body.to_owned(),
        timestamp,
    };
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().filter(|l| !l.is_empty()).collect();
    if lines.len() >= MAX_JOURNAL_ENTRIES {
        let start = lines.len() - (MAX_JOURNAL_ENTRIES - 1);
        lines = lines[start..].to_vec();
    }
    let Ok(new_line) = serde_json::to_string(&entry) else { return };
    let mut content = lines.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    content.push_str(&new_line);
    content.push('\n');
    if let Err(e) = std::fs::write(&path, &content) {
        log::warn!("notification journal: write failed: {e}");
    }
}

/// Load up to `limit` most-recent entries from the JSONL journal.
pub fn load_persisted_notifications(limit: usize) -> Vec<PersistedNotif> {
    let path = journal_path();
    let Ok(content) = std::fs::read_to_string(&path) else { return Vec::new() };
    let mut entries: Vec<PersistedNotif> = content
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    if entries.len() > limit {
        let start = entries.len() - limit;
        entries = entries[start..].to_vec();
    }
    entries
}

// ── Channel messages ──────────────────────────────────────────────────────────

/// Events the daemon thread sends to the GTK main thread.
#[derive(Debug, Clone)]
pub enum DaemonMsg {
    /// A new notification arrived (or replaces an existing one).
    Incoming {
        id: u32,
        app: String,
        summary: String,
        body: String,
        /// -1 = server default (use 5 s), 0 = never auto-expire, >0 = milliseconds.
        timeout_ms: i32,
    },
    /// The sender asked to programmatically close notification `id`.
    Close(u32),
}

// ── D-Bus interface implementation ────────────────────────────────────────────

struct NotificationsIface {
    tx: SyncSender<DaemonMsg>,
    next_id: Arc<AtomicU32>,
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationsIface {
    /// Called by every client that wants to display a notification.
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        _app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        _hints: HashMap<String, zbus::zvariant::OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id != 0 {
            replaces_id
        } else {
            self.next_id.fetch_add(1, Ordering::SeqCst)
        };
        let _ = self.tx.send(DaemonMsg::Incoming {
            id,
            app: app_name,
            summary,
            body,
            timeout_ms: expire_timeout,
        });
        id
    }

    async fn close_notification(&self, id: u32) {
        let _ = self.tx.send(DaemonMsg::Close(id));
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".into(), "persistence".into()]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "niri-shell".into(),
            "niri".into(),
            env!("CARGO_PKG_VERSION").into(),
            "1.2".into(),
        )
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

fn start_daemon(tx: SyncSender<DaemonMsg>) -> zbus::Result<zbus::blocking::Connection> {
    let iface = NotificationsIface {
        tx,
        next_id: Arc::new(AtomicU32::new(1)),
    };
    zbus::blocking::connection::Builder::session()?
        .name("org.freedesktop.Notifications")?
        .serve_at("/org/freedesktop/Notifications", iface)?
        .build()
}

/// Spawn the D-Bus notification daemon on a background thread.
///
/// `tx` receives every `DaemonMsg` on the GTK main thread (via a
/// `glib::timeout_add_local` drain loop in `shell.rs`).
/// Returns immediately; the daemon runs for the process lifetime.
pub fn spawn_daemon(tx: SyncSender<DaemonMsg>) {
    std::thread::spawn(move || match start_daemon(tx) {
        Ok(_conn) => {
            log::info!("notification daemon: registered on session bus");
            // Keep the connection alive indefinitely.
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        Err(e) => log::error!("notification daemon: failed to register: {e}"),
    });
}

// IPC types mirroring niri compositor's JSON wire protocol.
// Deserialized from newline-delimited JSON read from $NIRI_SOCKET.
// Fields may not all be consumed yet — they exist to match the wire format.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ── Compositor objects ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub id: u64,
    pub idx: u64,
    pub name: Option<String>,
    pub output: Option<String>,
    pub is_active: bool,
    pub is_focused: bool,
    pub active_window_id: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Window {
    pub id: u64,
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub workspace_id: Option<u64>,
    pub is_focused: bool,
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Subset of niri IPC events consumed by the shell.
/// Unknown event variants are ignored at the call sites by treating parse
/// failures as debug-level noise.
#[derive(Debug, Clone, Deserialize)]
pub enum NiriEvent {
    WorkspacesChanged { workspaces: Vec<Workspace> },
    WindowsChanged { windows: Vec<Window> },
    WindowFocusChanged { id: Option<u64> },
    WorkspaceActivated { id: u64, focused: bool },
    WindowOpenedOrChanged { window: Window },
    WindowClosed { id: u64 },
}

// ── Actions ───────────────────────────────────────────────────────────────────

/// niri actions that the shell may send (e.g. launch or focus a window).
#[derive(Debug, Clone, Serialize)]
pub enum NiriAction {
    Spawn { command: Vec<String> },
    FocusWindow { id: u64 },
}

// ── Wire request/reply wrappers ───────────────────────────────────────────────

/// Top-level IPC request sent to niri over the socket.
#[derive(Debug, Serialize)]
pub enum NiriRequest {
    EventStream,
    Action(NiriAction),
}

/// Top-level IPC reply wrapper received from niri.
#[derive(Debug, Deserialize)]
pub enum NiriReply {
    Ok(serde_json::Value),
    Err(String),
}

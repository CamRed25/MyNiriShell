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

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct OutputMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub is_preferred: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogicalOutput {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub transform: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NiriOutput {
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub physical_size: Option<[u32; 2]>,
    pub modes: Vec<OutputMode>,
    /// Index into `modes` for the currently active mode, if any.
    pub current_mode: Option<usize>,
    pub is_custom_mode: bool,
    pub vrr_supported: bool,
    pub vrr_enabled: bool,
    pub logical: Option<LogicalOutput>,
}

// ── Wire request/reply wrappers ───────────────────────────────────────────────

/// Top-level IPC request sent to niri over the socket.
#[derive(Debug, Serialize)]
pub enum NiriRequest {
    EventStream,
    Outputs,
    Action(NiriAction),
}

/// Top-level IPC reply wrapper received from niri.
#[derive(Debug, Deserialize)]
pub enum NiriReply {
    Ok(serde_json::Value),
    Err(String),
}

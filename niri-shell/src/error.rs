// Typed error definitions for the Niri shell backend.
// Each subsystem has its own variant so callers can match on failure causes precisely.

#![allow(dead_code)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShellError {
    #[error("window manager error: {0}")]
    WindowManager(#[from] WindowManagerError),

    #[error("panel error: {0}")]
    Panel(#[from] PanelError),

    #[error("launcher error: {0}")]
    Launcher(#[from] LauncherError),

    #[error("monitor error: {0}")]
    Monitor(#[from] MonitorError),

    #[error("input error: {0}")]
    Input(#[from] InputError),

    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}

#[derive(Debug, Error)]
pub enum WindowManagerError {
    #[error("failed to initialize window manager state")]
    InitFailed,
}

#[derive(Debug, Error)]
pub enum PanelError {
    #[error("failed to initialize panel state")]
    InitFailed,
}

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("failed to initialize launcher state")]
    InitFailed,
}

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("failed to initialize monitor state")]
    InitFailed,
}

#[derive(Debug, Error)]
pub enum InputError {
    #[error("failed to initialize input state")]
    InitFailed,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to initialize config state")]
    InitFailed,
    #[error("config file not found at {path}")]
    NotFound { path: String },
    #[error("config parse error: {0}")]
    ParseError(String),
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("failed to initialize protocol extension state")]
    InitFailed,
}

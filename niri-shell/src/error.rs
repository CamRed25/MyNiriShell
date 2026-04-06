// Typed error definitions for the niri shell.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShellError {
    #[error("gtk failed to initialize")]
    GtkInit,
    #[error("ipc error: {0}")]
    Ipc(#[from] IpcError),
}

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("$NIRI_SOCKET environment variable not set")]
    SocketEnvMissing,
    #[error("failed to connect to niri IPC socket: {0}")]
    Connect(std::io::Error),
    #[error("IPC send failed: {0}")]
    Send(String),
    #[error("IPC recv failed: {0}")]
    Recv(String),
    #[error("IPC JSON parse error: {0}")]
    Parse(String),
}

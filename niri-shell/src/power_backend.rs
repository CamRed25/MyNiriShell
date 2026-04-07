// Backend — zero GTK imports.
// Sends power management requests to logind via org.freedesktop.login1.

use thiserror::Error;
use zbus::blocking::Connection;

#[derive(Debug, Error)]
pub enum PowerError {
    #[error("failed to connect to system D-Bus: {0}")]
    Connect(zbus::Error),
    #[error("logind '{method}' call failed: {source}")]
    Call {
        method: &'static str,
        #[source]
        source: zbus::Error,
    },
}

fn logind_call(method: &'static str) -> Result<(), PowerError> {
    let conn = Connection::system().map_err(PowerError::Connect)?;
    conn.call_method(
        Some("org.freedesktop.login1"),
        "/org/freedesktop/login1",
        Some("org.freedesktop.login1.Manager"),
        method,
        &(false,), // interactive = false
    )
    .map_err(|e| PowerError::Call { method, source: e })?;
    Ok(())
}

/// Suspend the system via logind.
pub fn suspend() -> Result<(), PowerError> {
    logind_call("Suspend")
}

/// Reboot the system via logind.
pub fn reboot() -> Result<(), PowerError> {
    logind_call("Reboot")
}

/// Power off the system via logind.
pub fn poweroff() -> Result<(), PowerError> {
    logind_call("PowerOff")
}

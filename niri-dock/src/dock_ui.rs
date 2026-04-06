// Stub — UI implementation will be filled in by the dock-ui job.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockUiError {
    #[error("dock UI not yet implemented")]
    NotImplemented,
}

pub fn run() -> Result<(), DockUiError> {
    Err(DockUiError::NotImplemented)
}

// Stub panel UI — GTK4 application shell will be implemented in a follow-up task.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PanelUiError {
    #[error("panel ui not yet implemented")]
    NotImplemented,
}

pub fn run() -> Result<(), PanelUiError> {
    env_logger::init();
    log::info!("niri-panel starting (UI stub)");
    Ok(())
}

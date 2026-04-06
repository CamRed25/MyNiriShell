// Core shell logic for the Niri desktop environment backend.
// Integrates window management, panels, launcher, multi-monitor, input, config, and protocol extensions.

use crate::error::ShellError;

pub fn run() -> Result<(), ShellError> {
    env_logger::init();

    // Initialize core subsystems
    self::window_manager::init()?;
    self::panel::init()?;
    self::launcher::init()?;
    self::monitor::init()?;
    self::input::init()?;
    self::config::init()?;
    self::protocol::init()?;

    log::info!("Niri shell backend started.");
    Ok(())
}

pub mod window_manager;
pub mod panel;
pub mod launcher;
pub mod monitor;
pub mod input;
pub mod config;
pub mod protocol;

// Tests for Niri shell backend core and subsystems.
// Covers initialization and error handling for each subsystem.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_run_ok() {
        // Should initialize all subsystems without error
        assert!(run().is_ok());
    }

    #[test]
    fn test_window_manager_init_ok() {
        assert!(window_manager::init().is_ok());
    }

    #[test]
    fn test_panel_init_ok() {
        assert!(panel::init().is_ok());
    }

    #[test]
    fn test_launcher_init_ok() {
        assert!(launcher::init().is_ok());
    }

    #[test]
    fn test_monitor_init_ok() {
        assert!(monitor::init().is_ok());
    }

    #[test]
    fn test_input_init_ok() {
        assert!(input::init().is_ok());
    }

    #[test]
    fn test_config_init_ok() {
        assert!(config::init().is_ok());
    }

    #[test]
    fn test_protocol_init_ok() {
        assert!(protocol::init().is_ok());
    }
}

// Backend logic for the Niri panel. Zero GTK4 imports — pure Rust data and update methods.
// Public types are forward-declared for future D-Bus / IPC integration.
#![allow(dead_code)]

use thiserror::Error;

// ── Error types ──────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PanelError {
    #[error("workspace error: {0}")]
    Workspace(#[from] WorkspaceError),

    #[error("media error: {0}")]
    Media(#[from] MediaError),

    #[error("stats error: {0}")]
    Stats(#[from] StatsError),

    #[error("weather error: {0}")]
    Weather(#[from] WeatherError),

    #[error("notification error: {0}")]
    Notification(#[from] NotificationError),

    #[error("quick settings error: {0}")]
    QuickSettings(#[from] QuickSettingsError),
}

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("invalid workspace index: {0}")]
    InvalidIndex(usize),
}

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("no active media session")]
    NoSession,
}

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("cpu percentage out of range: {0}")]
    CpuOutOfRange(f32),

    #[error("memory value invalid: used {used} > total {total}")]
    MemoryUsedExceedsTotal { used: u64, total: u64 },

    #[error("volume percentage out of range: {0}")]
    VolumeOutOfRange(u8),
}

#[derive(Debug, Error)]
pub enum WeatherError {
    #[error("empty location string")]
    EmptyLocation,
}

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("notification id {0} not found")]
    NotFound(u32),
}

#[derive(Debug, Error)]
pub enum QuickSettingsError {
    #[error("brightness percentage out of range: {0}")]
    InvalidBrightness(u8),
}

// ── State types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct WorkspaceState {
    /// Index of the currently focused workspace (0-based).
    pub current_index: usize,
    /// Display names for each workspace.
    pub names: Vec<String>,
    /// Which workspace indices have at least one open window.
    pub occupied: Vec<bool>,
    /// Number of windows currently in the niri scratchpad (no workspace).
    pub scratchpad_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct MediaState {
    pub track_name: String,
    pub artist: String,
    /// `true` when music is playing, `false` when paused.
    pub playing: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SystemStats {
    /// CPU utilisation in percent (0.0–100.0).
    pub cpu_percent: f32,
    /// Memory in use, in bytes.
    pub memory_used: u64,
    /// Total available memory, in bytes.
    pub memory_total: u64,
    /// Network upload speed in bytes/s.
    pub network_up: u64,
    /// Network download speed in bytes/s.
    pub network_down: u64,
    /// Master volume level (0–100).
    pub volume_percent: u8,
}

#[derive(Debug, Clone, Default)]
pub struct WeatherState {
    /// Temperature in degrees Celsius.
    pub temperature: f32,
    /// Human-readable condition, e.g. "Partly Cloudy".
    pub condition: String,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct NotificationItem {
    pub id: u32,
    pub app: String,
    pub summary: String,
    pub body: String,
    /// Unix timestamp (seconds since epoch).
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default)]
pub struct QuickSettings {
    pub wifi_enabled: bool,
    /// SSID of the connected network, empty when disconnected.
    pub wifi_ssid: String,
    pub bluetooth_enabled: bool,
    /// Screen brightness (0–100).
    pub brightness_percent: u8,
}

// ── Aggregated panel state ────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct PanelState {
    pub workspaces: WorkspaceState,
    pub media: MediaState,
    pub stats: SystemStats,
    pub weather: WeatherState,
    pub notifications: Vec<NotificationItem>,
    pub quick_settings: QuickSettings,
}

impl PanelState {
    /// Returns a zeroed/default panel state.
    pub fn new() -> Self {
        Self::default()
    }

    // ── Workspace ──────────────────────────────────────────────────────────

    /// Update the number of windows in the scratchpad.
    pub fn update_scratchpad_count(&mut self, count: u32) {
        self.workspaces.scratchpad_count = count;
    }

    /// Replace the full workspace list and set the focused index.
    pub fn update_workspaces(
        &mut self,
        current_index: usize,
        names: Vec<String>,
        occupied: Vec<bool>,
    ) -> Result<(), WorkspaceError> {
        if current_index >= names.len() {
            return Err(WorkspaceError::InvalidIndex(current_index));
        }
        self.workspaces = WorkspaceState {
            current_index,
            names,
            occupied,
            scratchpad_count: self.workspaces.scratchpad_count,
        };
        Ok(())
    }

    // ── Media ──────────────────────────────────────────────────────────────

    /// Update the currently playing track.
    pub fn update_media(
        &mut self,
        track_name: impl Into<String>,
        artist: impl Into<String>,
        playing: bool,
    ) -> Result<(), MediaError> {
        self.media = MediaState {
            track_name: track_name.into(),
            artist: artist.into(),
            playing,
        };
        Ok(())
    }

    /// Toggle play/pause state. Returns an error when no track is loaded.
    pub fn toggle_playback(&mut self) -> Result<(), MediaError> {
        if self.media.track_name.is_empty() {
            return Err(MediaError::NoSession);
        }
        self.media.playing = !self.media.playing;
        Ok(())
    }

    // ── System stats ───────────────────────────────────────────────────────

    /// Update CPU, memory, network, and volume readings.
    pub fn update_stats(
        &mut self,
        cpu_percent: f32,
        memory_used: u64,
        memory_total: u64,
        network_up: u64,
        network_down: u64,
        volume_percent: u8,
    ) -> Result<(), StatsError> {
        if !(0.0..=100.0).contains(&cpu_percent) {
            return Err(StatsError::CpuOutOfRange(cpu_percent));
        }
        if memory_used > memory_total {
            return Err(StatsError::MemoryUsedExceedsTotal {
                used: memory_used,
                total: memory_total,
            });
        }
        if volume_percent > 100 {
            return Err(StatsError::VolumeOutOfRange(volume_percent));
        }
        self.stats = SystemStats {
            cpu_percent,
            memory_used,
            memory_total,
            network_up,
            network_down,
            volume_percent,
        };
        Ok(())
    }

    // ── Weather ────────────────────────────────────────────────────────────

    /// Update weather data.
    pub fn update_weather(
        &mut self,
        temperature: f32,
        condition: impl Into<String>,
        location: impl Into<String>,
    ) -> Result<(), WeatherError> {
        let location = location.into();
        if location.is_empty() {
            return Err(WeatherError::EmptyLocation);
        }
        self.weather = WeatherState { temperature, condition: condition.into(), location };
        Ok(())
    }

    // ── Notifications ─────────────────────────────────────────────────────

    /// Append a new notification, replacing any existing entry with the same id.
    pub fn add_notification(
        &mut self,
        id: u32,
        app: impl Into<String>,
        summary: impl Into<String>,
        body: impl Into<String>,
        timestamp: u64,
    ) -> Result<(), NotificationError> {
        self.notifications.retain(|n| n.id != id);
        self.notifications.push(NotificationItem {
            id,
            app: app.into(),
            summary: summary.into(),
            body: body.into(),
            timestamp,
        });
        Ok(())
    }

    /// Remove the notification with the given id.
    pub fn dismiss_notification(&mut self, id: u32) -> Result<(), NotificationError> {
        let before = self.notifications.len();
        self.notifications.retain(|n| n.id != id);
        if self.notifications.len() == before {
            return Err(NotificationError::NotFound(id));
        }
        Ok(())
    }

    /// Return notification counts per app name, for dock badge display.
    pub fn notification_counts_by_app(&self) -> std::collections::HashMap<String, u32> {
        let mut map = std::collections::HashMap::new();
        for n in &self.notifications {
            *map.entry(n.app.clone()).or_insert(0) += 1;
        }
        map
    }

    // ── Quick settings ─────────────────────────────────────────────────────

    /// Update connectivity and brightness settings.
    pub fn update_quick_settings(
        &mut self,
        wifi_enabled: bool,
        wifi_ssid: impl Into<String>,
        bluetooth_enabled: bool,
        brightness_percent: u8,
    ) -> Result<(), QuickSettingsError> {
        if brightness_percent > 100 {
            return Err(QuickSettingsError::InvalidBrightness(brightness_percent));
        }
        self.quick_settings = QuickSettings {
            wifi_enabled,
            wifi_ssid: wifi_ssid.into(),
            bluetooth_enabled,
            brightness_percent,
        };
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> PanelState {
        PanelState::new()
    }

    #[test]
    fn test_default_state() {
        let s = state();
        assert_eq!(s.workspaces.current_index, 0);
        assert!(s.notifications.is_empty());
    }

    #[test]
    fn test_update_workspaces_ok() {
        let mut s = state();
        assert!(s
            .update_workspaces(1, vec!["1".into(), "2".into(), "3".into()], vec![true, false, true])
            .is_ok());
        assert_eq!(s.workspaces.current_index, 1);
    }

    #[test]
    fn test_update_workspaces_invalid_index() {
        let mut s = state();
        let result = s.update_workspaces(5, vec!["1".into()], vec![false]);
        assert!(matches!(result, Err(WorkspaceError::InvalidIndex(5))));
    }

    #[test]
    fn test_update_media_ok() {
        let mut s = state();
        assert!(s.update_media("Track", "Artist", true).is_ok());
        assert!(s.media.playing);
    }

    #[test]
    fn test_toggle_playback_no_session() {
        let mut s = state();
        assert!(matches!(s.toggle_playback(), Err(MediaError::NoSession)));
    }

    #[test]
    fn test_toggle_playback_ok() {
        let mut s = state();
        s.update_media("T", "A", true).unwrap();
        s.toggle_playback().unwrap();
        assert!(!s.media.playing);
    }

    #[test]
    fn test_update_stats_ok() {
        let mut s = state();
        assert!(s.update_stats(42.0, 4_000_000, 8_000_000, 1024, 2048, 80).is_ok());
        assert_eq!(s.stats.volume_percent, 80);
    }

    #[test]
    fn test_update_stats_invalid_cpu() {
        let mut s = state();
        assert!(matches!(
            s.update_stats(101.0, 0, 0, 0, 0, 0),
            Err(StatsError::CpuOutOfRange(_))
        ));
    }

    #[test]
    fn test_update_stats_invalid_memory() {
        let mut s = state();
        assert!(matches!(
            s.update_stats(50.0, 10, 5, 0, 0, 50),
            Err(StatsError::MemoryUsedExceedsTotal { .. })
        ));
    }

    #[test]
    fn test_update_weather_ok() {
        let mut s = state();
        assert!(s.update_weather(18.5, "Sunny", "London").is_ok());
        assert_eq!(s.weather.location, "London");
    }

    #[test]
    fn test_update_weather_empty_location() {
        let mut s = state();
        assert!(matches!(s.update_weather(0.0, "Clear", ""), Err(WeatherError::EmptyLocation)));
    }

    #[test]
    fn test_add_and_dismiss_notification() {
        let mut s = state();
        s.add_notification(1, "app", "summary", "body", 1000).unwrap();
        assert_eq!(s.notifications.len(), 1);
        s.dismiss_notification(1).unwrap();
        assert!(s.notifications.is_empty());
    }

    #[test]
    fn test_dismiss_notification_not_found() {
        let mut s = state();
        assert!(matches!(s.dismiss_notification(99), Err(NotificationError::NotFound(99))));
    }

    #[test]
    fn test_add_notification_replaces_duplicate_id() {
        let mut s = state();
        s.add_notification(1, "app", "old", "body", 1000).unwrap();
        s.add_notification(1, "app", "new", "body", 2000).unwrap();
        assert_eq!(s.notifications.len(), 1);
        assert_eq!(s.notifications[0].summary, "new");
    }

    #[test]
    fn test_update_quick_settings_ok() {
        let mut s = state();
        assert!(s.update_quick_settings(true, "HomeWifi", false, 75).is_ok());
        assert_eq!(s.quick_settings.wifi_ssid, "HomeWifi");
    }

    #[test]
    fn test_update_quick_settings_invalid_brightness() {
        let mut s = state();
        assert!(matches!(
            s.update_quick_settings(false, "", false, 101),
            Err(QuickSettingsError::InvalidBrightness(101))
        ));
    }
}

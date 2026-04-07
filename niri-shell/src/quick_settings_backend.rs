// BACKEND ONLY — zero GTK/glib imports.
//
// Describes all Quick-Settings tile states; provides query and toggle helpers
// that are called synchronously on the GTK main thread when the user interacts
// with the Quick Settings overlay.

use std::fs;
use std::process::{Child, Command};

use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum QsError {
    #[error("brightness error: {0}")]
    Brightness(String),
    #[error("subprocess error: {0}")]
    Subprocess(String),
}

// ── Power profile ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerProfile {
    PowerSaver,
    #[default]
    Balanced,
    Performance,
}

impl PowerProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::PowerSaver => "Power Saver",
            Self::Balanced => "Balanced",
            Self::Performance => "Performance",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::PowerSaver => "power-saver",
            Self::Balanced => "balanced",
            Self::Performance => "performance",
        }
    }

    /// Cycle to the next profile (PowerSaver → Balanced → Performance → PowerSaver).
    pub fn next(self) -> Self {
        match self {
            Self::PowerSaver => Self::Balanced,
            Self::Balanced => Self::Performance,
            Self::Performance => Self::PowerSaver,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

/// Snapshot of all Quick Settings tile states.
#[derive(Debug, Clone)]
pub struct QsState {
    pub wifi_active: bool,
    pub wifi_ssid: String,
    pub ethernet_active: bool,
    pub vpn_active: bool,
    pub bt_active: bool,
    /// Night Light (gammastep) — tracked in-process, not queried from the OS.
    pub night_light: bool,
    /// Do Not Disturb — suppresses toast popups; tracked in-process.
    pub dnd: bool,
    pub kb_layout: String,
    pub mic_muted: bool,
    pub power_profile: PowerProfile,
    /// Idle inhibitor — tracked in-process via the child process handle.
    pub idle_inhibited: bool,
    pub brightness: u8,
    pub volume: u8,
}

impl Default for QsState {
    fn default() -> Self {
        Self {
            wifi_active: false,
            wifi_ssid: String::new(),
            ethernet_active: false,
            vpn_active: false,
            bt_active: false,
            night_light: false,
            dnd: false,
            kb_layout: String::new(),
            mic_muted: false,
            power_profile: PowerProfile::default(),
            idle_inhibited: false,
            brightness: 100,
            volume: 50,
        }
    }
}

impl QsState {
    /// Query all external state. Failures fall back to defaults silently.
    pub fn load() -> Self {
        Self {
            wifi_active: query_wifi_active(),
            wifi_ssid: query_wifi_ssid(),
            ethernet_active: query_ethernet_active(),
            vpn_active: query_vpn_active(),
            bt_active: query_bt_active(),
            night_light: false,
            dnd: false,
            kb_layout: query_kb_layout(),
            mic_muted: query_mic_muted(),
            power_profile: query_power_profile(),
            idle_inhibited: false,
            brightness: read_brightness(),
            volume: read_volume(),
        }
    }
}

// ── Query helpers ─────────────────────────────────────────────────────────────

fn run_cmd(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

pub fn query_wifi_active() -> bool {
    run_cmd("nmcli", &["-t", "-f", "WIFI", "g"])
        .map(|s| s == "enabled")
        .unwrap_or(false)
}

pub fn query_wifi_ssid() -> String {
    run_cmd("nmcli", &["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
        .and_then(|out| {
            out.lines()
                .find(|l| l.starts_with("yes:"))
                .map(|l| l[4..].to_owned())
        })
        .unwrap_or_default()
}

pub fn query_ethernet_active() -> bool {
    run_cmd("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE", "dev"])
        .map(|out| {
            out.lines().any(|l| {
                let p: Vec<&str> = l.splitn(3, ':').collect();
                p.get(1) == Some(&"ethernet") && p.get(2) == Some(&"connected")
            })
        })
        .unwrap_or(false)
}

pub fn query_vpn_active() -> bool {
    run_cmd("nmcli", &["-t", "-f", "TYPE", "con", "show", "--active"])
        .map(|out| out.lines().any(|l| l.contains("vpn")))
        .unwrap_or(false)
}

pub fn query_bt_active() -> bool {
    run_cmd("rfkill", &["list", "bluetooth"])
        .map(|out| out.lines().any(|l| l.contains("Soft blocked: no")))
        .unwrap_or(false)
}

pub fn query_kb_layout() -> String {
    run_cmd("xkb-switch", &["-p"]).unwrap_or_default()
}

pub fn query_mic_muted() -> bool {
    run_cmd("pactl", &["get-source-mute", "@DEFAULT_SOURCE@"])
        .map(|s| s.contains("yes"))
        .unwrap_or(false)
}

pub fn query_power_profile() -> PowerProfile {
    match run_cmd("powerprofilesctl", &["get"]).as_deref() {
        Some("power-saver") => PowerProfile::PowerSaver,
        Some("performance") => PowerProfile::Performance,
        _ => PowerProfile::Balanced,
    }
}

pub fn read_brightness() -> u8 {
    // Try sysfs backlight (laptops / integrated GPU).
    let base = std::path::Path::new("/sys/class/backlight");
    if let Ok(mut entries) = fs::read_dir(base) {
        if let Some(Ok(entry)) = entries.next() {
            let dir = entry.path();
            let max: u64 = fs::read_to_string(dir.join("max_brightness"))
                .ok().and_then(|s| s.trim().parse().ok()).unwrap_or(255);
            let cur: u64 = fs::read_to_string(dir.join("brightness"))
                .ok().and_then(|s| s.trim().parse().ok()).unwrap_or(max);
            if max > 0 {
                return ((cur * 100) / max).min(100) as u8;
            }
        }
    }
    // Try ddcutil (external monitors via DDC/CI).
    if let Some(out) = run_cmd("ddcutil", &["getvcp", "10", "--brief"]) {
        // Output: "VCP 10 C <current> <max>"
        let parts: Vec<&str> = out.split_whitespace().collect();
        if parts.len() >= 5 {
            if let (Ok(cur), Ok(max)) = (parts[3].parse::<u64>(), parts[4].parse::<u64>()) {
                if max > 0 {
                    return ((cur * 100) / max).min(100) as u8;
                }
            }
        }
    }
    // No brightness device found — return sentinel 255 to signal "unavailable".
    255
}

/// `true` when at least one brightness backend is present on this machine.
pub fn brightness_available() -> bool {
    read_brightness() != 255
}

pub fn read_volume() -> u8 {
    run_cmd("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])
        .and_then(|s| {
            // Output looks like: "Volume: front-left: 65536 /  50% / ..."
            s.split('%').next()?.split_whitespace().last()?.parse().ok()
        })
        .unwrap_or(50)
}

// ── Toggle / set operations ────────────────────────────────────────────────────

pub fn toggle_wifi(current: bool) -> Result<bool, QsError> {
    let arg = if current { "off" } else { "on" };
    Command::new("nmcli")
        .args(["radio", "wifi", arg])
        .status()
        .map_err(|e| QsError::Subprocess(e.to_string()))?;
    Ok(!current)
}

pub fn toggle_bt(current: bool) -> Result<bool, QsError> {
    let arg = if current { "block" } else { "unblock" };
    Command::new("rfkill")
        .args([arg, "bluetooth"])
        .status()
        .map_err(|e| QsError::Subprocess(e.to_string()))?;
    Ok(!current)
}

/// Spawn / kill a `gammastep -O 4500` process for Night Light.
/// The child handle is stored so we can kill it when toggled off.
pub fn toggle_night_light(
    current: bool,
    child: &mut Option<Child>,
) -> Result<bool, QsError> {
    if current {
        if let Some(mut c) = child.take() {
            let _ = c.kill();
        }
        Ok(false)
    } else {
        let c = Command::new("gammastep")
            .args(["-O", "4500"])
            .spawn()
            .map_err(|e| QsError::Subprocess(e.to_string()))?;
        *child = Some(c);
        Ok(true)
    }
}

pub fn toggle_mic_mute() -> Result<bool, QsError> {
    Command::new("pactl")
        .args(["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])
        .status()
        .map_err(|e| QsError::Subprocess(e.to_string()))?;
    Ok(query_mic_muted())
}

pub fn cycle_power_profile(current: PowerProfile) -> Result<PowerProfile, QsError> {
    let next = current.next();
    Command::new("powerprofilesctl")
        .args(["set", next.id()])
        .status()
        .map_err(|e| QsError::Subprocess(e.to_string()))?;
    Ok(next)
}

/// Spawn / kill `systemd-inhibit --what=idle … sleep infinity` for idle inhibition.
pub fn toggle_idle_inhibitor(
    current: bool,
    child: &mut Option<Child>,
) -> Result<bool, QsError> {
    if current {
        if let Some(mut c) = child.take() {
            let _ = c.kill();
        }
        Ok(false)
    } else {
        let c = Command::new("systemd-inhibit")
            .args(["--what=idle", "--who=niri-shell", "--why=user-request", "sleep", "infinity"])
            .spawn()
            .map_err(|e| QsError::Subprocess(e.to_string()))?;
        *child = Some(c);
        Ok(true)
    }
}

pub fn set_brightness(percent: u8) -> Result<(), QsError> {
    // Try sysfs backlight first.
    let base = std::path::Path::new("/sys/class/backlight");
    if let Ok(mut entries) = fs::read_dir(base) {
        if let Some(Ok(entry)) = entries.next() {
            let dir = entry.path();
            let max: u64 = fs::read_to_string(dir.join("max_brightness"))
                .map_err(|e| QsError::Brightness(e.to_string()))?
                .trim().parse()
                .map_err(|e: std::num::ParseIntError| QsError::Brightness(e.to_string()))?;
            let target = (u64::from(percent) * max / 100).min(max);
            return fs::write(dir.join("brightness"), target.to_string())
                .map_err(|e| QsError::Brightness(e.to_string()));
        }
    }
    // Try ddcutil (external monitor DDC/CI).
    if Command::new("ddcutil")
        .args(["setvcp", "10", &percent.to_string()])
        .status().is_ok()
    {
        return Ok(());
    }
    Err(QsError::Brightness("no brightness backend available (no backlight or ddcutil)".into()))
}

pub fn set_volume_abs(percent: u8) -> Result<(), QsError> {
    Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{percent}%")])
        .status()
        .map_err(|e| QsError::Subprocess(e.to_string()))?;
    Ok(())
}

/// Fire-and-forget launch of a shell command.
pub fn launch(cmd: &str) {
    let _ = Command::new("sh").args(["-c", cmd]).spawn();
}

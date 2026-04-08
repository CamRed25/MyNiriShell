// settings_backend.rs — Pure Rust config model for the niri settings GUI.
//
// Persists to ~/.config/niri-shell/settings.json (serde state).
// On every save, generates ~/.config/niri-shell/settings.kdl (KDL include)
// and ensures that file is included from ~/.config/niri/config.kdl.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),
}

// ── Config types ──────────────────────────────────────────────────────────────

/// Root settings configuration managed by the settings GUI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsConfig {
    #[serde(default)]
    pub input: InputSettings,
    #[serde(default)]
    pub layout: LayoutSettings,
    #[serde(default)]
    pub misc: MiscSettings,
    #[serde(default)]
    pub anim: AnimSettings,
    #[serde(default)]
    pub gestures: GestureSettings,
    #[serde(default)]
    pub workspaces: Vec<NamedWorkspace>,
    #[serde(default)]
    pub recent_windows: RecentWindowsSettings,
    #[serde(default)]
    pub debug: DebugSettings,
    #[serde(default)]
    pub outputs: Vec<OutputConfig>,
    #[serde(default)]
    pub window_rules: Vec<WindowRule>,
    #[serde(default)]
    pub layer_rules: Vec<LayerRule>,
    #[serde(default)]
    pub switch_events: SwitchEventsSettings,
    #[serde(default)]
    pub binds: Vec<Keybind>,
}

// ── Rules & switch events ────────────────────────────────────────────────────

/// Shared block-out-from value used by window and layer rules.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum BlockOutFrom {
    #[default]
    None,
    Screencast,
    ScreenCapture,
}

impl BlockOutFrom {
    pub fn label_variants() -> &'static [&'static str] {
        &["none", "screencast", "screen-capture"]
    }
    pub fn from_index(i: u32) -> Self {
        match i { 1 => Self::Screencast, 2 => Self::ScreenCapture, _ => Self::None }
    }
    pub fn to_index(&self) -> u32 {
        match self { Self::None => 0, Self::Screencast => 1, Self::ScreenCapture => 2 }
    }
    pub fn as_kdl_str(&self) -> &'static str {
        match self { Self::Screencast => "screencast", Self::ScreenCapture => "screen-capture", Self::None => "" }
    }
}

/// Tri-state: use niri default / force on / force off.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TriState {
    #[default]
    Default,
    On,
    Off,
}

impl TriState {
    pub fn label_variants() -> &'static [&'static str] {
        &["default", "on", "off"]
    }
    pub fn from_index(i: u32) -> Self {
        match i { 1 => Self::On, 2 => Self::Off, _ => Self::Default }
    }
    pub fn to_index(&self) -> u32 {
        match self { Self::Default => 0, Self::On => 1, Self::Off => 2 }
    }
}

/// A single window rule.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowRule {
    // ── match criteria ────────────────────────────────────────────────────────
    /// app-id regex, empty = match all
    #[serde(default)]
    pub match_app_id: String,
    /// title regex, empty = match all
    #[serde(default)]
    pub match_title: String,
    /// at-startup matcher
    #[serde(default)]
    pub match_at_startup: TriState,
    // ── opening properties ────────────────────────────────────────────────────
    #[serde(default)]
    pub open_maximized: TriState,
    #[serde(default)]
    pub open_fullscreen: TriState,
    #[serde(default)]
    pub open_floating: TriState,
    #[serde(default)]
    pub open_focused: TriState,
    /// empty = don't write
    #[serde(default)]
    pub open_on_output: String,
    /// empty = don't write
    #[serde(default)]
    pub open_on_workspace: String,
    // ── dynamic properties ────────────────────────────────────────────────────
    /// 0.0 = don't write; range 0.0–1.0
    #[serde(default)]
    pub opacity: f64,
    #[serde(default)]
    pub block_out_from: BlockOutFrom,
    #[serde(default)]
    pub draw_border_with_background: TriState,
    /// 0.0 = don't write
    #[serde(default)]
    pub geometry_corner_radius: f64,
    #[serde(default)]
    pub clip_to_geometry: TriState,
    #[serde(default)]
    pub variable_refresh_rate: TriState,
    /// 0 = don't write
    #[serde(default)]
    pub min_width: u32,
    #[serde(default)]
    pub max_width: u32,
    #[serde(default)]
    pub min_height: u32,
    #[serde(default)]
    pub max_height: u32,
    /// 0.0 = don't write
    #[serde(default)]
    pub scroll_factor: f64,
}

/// A single layer-shell rule.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerRule {
    /// namespace regex, empty = match all surfaces
    #[serde(default)]
    pub match_namespace: String,
    #[serde(default)]
    pub match_at_startup: TriState,
    /// 0.0 = don't write
    #[serde(default)]
    pub opacity: f64,
    #[serde(default)]
    pub block_out_from: BlockOutFrom,
    #[serde(default)]
    pub shadow: TriState,
    /// 0.0 = don't write
    #[serde(default)]
    pub geometry_corner_radius: f64,
    #[serde(default)]
    pub place_within_backdrop: TriState,
}

/// Configuration for `switch-events {}`.
/// Each event holds the argv of the spawn command, or empty vec = no binding.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SwitchEventsSettings {
    /// Command for lid-close; empty = no binding
    #[serde(default)]
    pub lid_close: Vec<String>,
    #[serde(default)]
    pub lid_open: Vec<String>,
    #[serde(default)]
    pub tablet_mode_on: Vec<String>,
    #[serde(default)]
    pub tablet_mode_off: Vec<String>,
}

// ── Key bindings ─────────────────────────────────────────────────────────────

/// Full ordered list of niri action names available for keybinding.
pub const NIRI_ACTIONS: &[&str] = &[
    // ── Launch ───────────────────────────────────────────────────────────────
    "spawn",
    "spawn-sh",
    // ── Window ───────────────────────────────────────────────────────────────
    "close-window",
    "fullscreen-window",
    "toggle-windowed-fullscreen",
    "toggle-window-floating",
    "move-window-to-floating",
    "move-window-to-tiling",
    "focus-floating",
    "focus-tiling",
    "switch-focus-between-floating-and-tiling",
    "toggle-window-rule-opacity",
    "toggle-window-urgent",
    "set-window-urgent",
    "unset-window-urgent",
    "maximize-window-to-edges",
    "set-window-width",
    "set-window-height",
    "reset-window-height",
    "switch-preset-window-width",
    "switch-preset-window-width-back",
    "switch-preset-window-height",
    "switch-preset-window-height-back",
    "move-window-down",
    "move-window-up",
    "move-window-down-or-to-workspace-down",
    "move-window-up-or-to-workspace-up",
    "move-window-to-workspace",
    "move-window-to-workspace-down",
    "move-window-to-workspace-up",
    "consume-or-expel-window-left",
    "consume-or-expel-window-right",
    "consume-window-into-column",
    "expel-window-from-column",
    "swap-window-left",
    "swap-window-right",
    "move-floating-window",
    // ── Focus ────────────────────────────────────────────────────────────────
    "focus-window-previous",
    "focus-window-down",
    "focus-window-up",
    "focus-window-down-or-column-left",
    "focus-window-down-or-column-right",
    "focus-window-up-or-column-left",
    "focus-window-up-or-column-right",
    "focus-window-or-monitor-up",
    "focus-window-or-monitor-down",
    "focus-window-or-workspace-down",
    "focus-window-or-workspace-up",
    "focus-window-top",
    "focus-window-bottom",
    "focus-window-down-or-top",
    "focus-window-up-or-bottom",
    // ── Column ───────────────────────────────────────────────────────────────
    "focus-column-left",
    "focus-column-right",
    "focus-column-first",
    "focus-column-last",
    "focus-column-right-or-first",
    "focus-column-left-or-last",
    "focus-column-or-monitor-left",
    "focus-column-or-monitor-right",
    "move-column-left",
    "move-column-right",
    "move-column-to-first",
    "move-column-to-last",
    "move-column-left-or-to-monitor-left",
    "move-column-right-or-to-monitor-right",
    "move-column-to-monitor-left",
    "move-column-to-monitor-right",
    "move-column-to-monitor-down",
    "move-column-to-monitor-up",
    "move-column-to-monitor-previous",
    "move-column-to-monitor-next",
    "move-column-to-workspace",
    "move-column-to-workspace-down",
    "move-column-to-workspace-up",
    "toggle-column-tabbed-display",
    "set-column-display",
    "set-column-width",
    "switch-preset-column-width",
    "switch-preset-column-width-back",
    "maximize-column",
    "expand-column-to-available-width",
    // ── Workspace ────────────────────────────────────────────────────────────
    "focus-workspace",
    "focus-workspace-down",
    "focus-workspace-up",
    "focus-workspace-previous",
    "move-workspace-to-monitor-left",
    "move-workspace-to-monitor-right",
    "move-workspace-to-monitor-down",
    "move-workspace-to-monitor-up",
    "move-workspace-to-monitor-previous",
    "move-workspace-to-monitor-next",
    "move-workspace-to-monitor",
    // ── Overlay / UI ─────────────────────────────────────────────────────────
    "toggle-overview",
    "open-overview",
    "close-overview",
    "show-hotkey-overlay",
    "toggle-keyboard-shortcuts-inhibit",
    // ── Screenshot ───────────────────────────────────────────────────────────
    "screenshot",
    "screenshot-screen",
    "screenshot-window",
    "do-screen-transition",
    // ── Layout / misc ────────────────────────────────────────────────────────
    "switch-layout",
    "power-off-monitors",
    "power-on-monitors",
    "set-dynamic-cast-window",
    "set-dynamic-cast-monitor",
    "clear-dynamic-cast-target",
    "load-config-file",
    // ── Debug ────────────────────────────────────────────────────────────────
    "toggle-debug-tint",
    "debug-toggle-opaque-regions",
    "debug-toggle-damage",
    // ── Session ──────────────────────────────────────────────────────────────
    "quit",
];

/// Returns a short hint describing what to put in `action_args` for a given action.
/// Returns an empty string for no-argument actions.
pub fn action_args_hint(action: &str) -> &'static str {
    match action {
        "spawn" => "program  arg1  arg2 …",
        "spawn-sh" => "shell command string",
        "focus-workspace"
        | "move-window-to-workspace"
        | "move-column-to-workspace" => "workspace index  (1, 2, …)",
        "set-column-width" | "set-window-width" => "size  e.g. 50%  or  1200",
        "set-window-height" => "size  e.g. 50%  or  800",
        "set-column-display" => "normal  or  tabbed",
        "switch-layout" => "next  /  prev  /  index",
        "do-screen-transition" => "delay-ms=250  (optional)",
        "quit" => "skip-confirmation=true  (optional)",
        "move-workspace-to-monitor"
        | "move-column-to-monitor" => "monitor name or index",
        "move-floating-window" => "dx dy  (pixels, e.g. 0 50)",
        _ => "",
    }
}

/// Returns `true` if the given action takes mandatory or useful arguments.
pub fn action_needs_args(action: &str) -> bool {
    matches!(
        action,
        "spawn"
            | "spawn-sh"
            | "focus-workspace"
            | "move-window-to-workspace"
            | "move-column-to-workspace"
            | "move-workspace-to-monitor"
            | "move-column-to-monitor"
            | "set-column-width"
            | "set-window-width"
            | "set-window-height"
            | "set-column-display"
            | "switch-layout"
            | "move-floating-window"
    )
}

fn default_true() -> bool {
    true
}

/// A single keybinding entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybind {
    /// Full key combo as niri expects, e.g. `"Mod+T"`, `"XF86AudioMute"`.
    #[serde(default)]
    pub key: String,
    /// Action name in kebab-case, e.g. `"spawn"`, `"close-window"`.
    #[serde(default)]
    pub action: String,
    /// Arguments for actions that need them (space-separated for `spawn`,
    /// whole string for `spawn-sh`, index for `focus-workspace`, etc.).
    #[serde(default)]
    pub action_args: String,
    /// Whether the bind repeats while held (`repeat=false` in KDL when `false`).
    /// Defaults to `true`.
    #[serde(default = "default_true")]
    pub repeat: bool,
    /// Rate-limiting cooldown in milliseconds. `0` = none.
    #[serde(default)]
    pub cooldown_ms: u32,
    /// For `spawn` only: also fire when the session is locked.
    #[serde(default)]
    pub allow_when_locked: bool,
}

impl Default for Keybind {
    fn default() -> Self {
        Self {
            key: String::new(),
            action: "spawn".into(),
            action_args: String::new(),
            repeat: true,
            cooldown_ms: 0,
            allow_when_locked: false,
        }
    }
}

// ── Output enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum OutputTransform {
    #[default]
    Normal,
    #[serde(rename = "90")]
    Rotate90,
    #[serde(rename = "180")]
    Rotate180,
    #[serde(rename = "270")]
    Rotate270,
    #[serde(rename = "flipped")]
    Flipped,
    #[serde(rename = "flipped-90")]
    Flipped90,
    #[serde(rename = "flipped-180")]
    Flipped180,
    #[serde(rename = "flipped-270")]
    Flipped270,
}

impl OutputTransform {
    pub fn as_kdl_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Rotate90 => "90",
            Self::Rotate180 => "180",
            Self::Rotate270 => "270",
            Self::Flipped => "flipped",
            Self::Flipped90 => "flipped-90",
            Self::Flipped180 => "flipped-180",
            Self::Flipped270 => "flipped-270",
        }
    }

    pub fn label_variants() -> &'static [&'static str] {
        &["normal", "90°", "180°", "270°", "flipped", "flipped 90°", "flipped 180°", "flipped 270°"]
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            1 => Self::Rotate90,
            2 => Self::Rotate180,
            3 => Self::Rotate270,
            4 => Self::Flipped,
            5 => Self::Flipped90,
            6 => Self::Flipped180,
            7 => Self::Flipped270,
            _ => Self::Normal,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            Self::Normal => 0,
            Self::Rotate90 => 1,
            Self::Rotate180 => 2,
            Self::Rotate270 => 3,
            Self::Flipped => 4,
            Self::Flipped90 => 5,
            Self::Flipped180 => 6,
            Self::Flipped270 => 7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum VrrMode {
    #[default]
    Default, // don't write — use niri's default (off)
    On,
    Off,
    OnDemand,
}

impl VrrMode {
    pub fn as_kdl_str(&self) -> &'static str {
        match self {
            Self::Default | Self::Off => "off",
            Self::On => "on",
            Self::OnDemand => "on-demand",
        }
    }

    pub fn label_variants() -> &'static [&'static str] {
        &["default", "on", "off", "on-demand"]
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            1 => Self::On,
            2 => Self::Off,
            3 => Self::OnDemand,
            _ => Self::Default,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            Self::Default => 0,
            Self::On => 1,
            Self::Off => 2,
            Self::OnDemand => 3,
        }
    }
}

/// Persisted configuration for one physical output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Connector name, e.g. "HDMI-A-1"
    pub name: String,
    /// If true, write `off` inside the output block.
    #[serde(default)]
    pub off: bool,
    /// Mode width in pixels, 0 = niri auto-pick.
    #[serde(default)]
    pub mode_width: u32,
    /// Mode height in pixels, 0 = niri auto-pick.
    #[serde(default)]
    pub mode_height: u32,
    /// Refresh rate in mHz (e.g. 59940), 0 = pick best for mode.
    #[serde(default)]
    pub mode_refresh_mhz: u32,
    /// Scale factor, 0.0 = niri auto.
    #[serde(default)]
    pub scale: f64,
    /// Display transform.
    #[serde(default)]
    pub transform: OutputTransform,
    /// Whether position is overridden.
    #[serde(default)]
    pub position_set: bool,
    #[serde(default)]
    pub position_x: i32,
    #[serde(default)]
    pub position_y: i32,
    /// Variable refresh rate mode.
    #[serde(default)]
    pub vrr: VrrMode,
}

impl OutputConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            off: false,
            mode_width: 0,
            mode_height: 0,
            mode_refresh_mhz: 0,
            scale: 0.0,
            transform: OutputTransform::Normal,
            position_set: false,
            position_x: 0,
            position_y: 0,
            vrr: VrrMode::Default,
        }
    }
}

// ── Input enums ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TrackLayout {
    #[default]
    Global,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum AccelProfile {
    #[default]
    Default,
    Adaptive,
    Flat,
}

impl AccelProfile {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::Adaptive => Some("adaptive"),
            Self::Flat => Some("flat"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TouchpadScrollMethod {
    #[default]
    Default,
    TwoFinger,
    Edge,
    OnButtonDown,
    NoScroll,
}

impl TouchpadScrollMethod {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::TwoFinger => Some("two-finger"),
            Self::Edge => Some("edge"),
            Self::OnButtonDown => Some("on-button-down"),
            Self::NoScroll => Some("no-scroll"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum MouseScrollMethod {
    #[default]
    Default,
    NoScroll,
    TwoFinger,
    Edge,
    OnButtonDown,
}

impl MouseScrollMethod {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::NoScroll => Some("no-scroll"),
            Self::TwoFinger => Some("two-finger"),
            Self::Edge => Some("edge"),
            Self::OnButtonDown => Some("on-button-down"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TapButtonMap {
    #[default]
    Default,
    LeftRightMiddle,
    LeftMiddleRight,
}

impl TapButtonMap {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::LeftRightMiddle => Some("left-right-middle"),
            Self::LeftMiddleRight => Some("left-middle-right"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ClickMethod {
    #[default]
    Default,
    ButtonAreas,
    Clickfinger,
}

impl ClickMethod {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::ButtonAreas => Some("button-areas"),
            Self::Clickfinger => Some("clickfinger"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum DragSetting {
    #[default]
    Default,
    Enabled,
    Disabled,
}

/// Settings from the `input {}` section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputSettings {
    // ── keyboard ─────────────────────────────────────────────────────────────
    pub numlock: bool,
    #[serde(default)]
    pub keyboard_xkb_layout: String,
    #[serde(default)]
    pub keyboard_xkb_variant: String,
    #[serde(default)]
    pub keyboard_xkb_model: String,
    #[serde(default)]
    pub keyboard_xkb_rules: String,
    #[serde(default)]
    pub keyboard_xkb_options: String,
    #[serde(default)]
    pub keyboard_repeat_delay: u32, // 0 = niri default (600 ms)
    #[serde(default)]
    pub keyboard_repeat_rate: u32,  // 0 = niri default (25 cps)
    #[serde(default)]
    pub keyboard_track_layout: TrackLayout,
    // ── touchpad ─────────────────────────────────────────────────────────────
    pub touchpad_tap: bool,
    pub touchpad_natural_scroll: bool,
    pub touchpad_dwt: bool,
    #[serde(default)]
    pub touchpad_off: bool,
    #[serde(default)]
    pub touchpad_dwtp: bool,
    #[serde(default)]
    pub touchpad_drag: DragSetting,
    #[serde(default)]
    pub touchpad_drag_lock: bool,
    #[serde(default)]
    pub touchpad_accel_speed: f64,
    #[serde(default)]
    pub touchpad_accel_profile: AccelProfile,
    #[serde(default)]
    pub touchpad_scroll_method: TouchpadScrollMethod,
    #[serde(default)]
    pub touchpad_tap_button_map: TapButtonMap,
    #[serde(default)]
    pub touchpad_click_method: ClickMethod,
    #[serde(default)]
    pub touchpad_disabled_on_external_mouse: bool,
    #[serde(default)]
    pub touchpad_left_handed: bool,
    #[serde(default)]
    pub touchpad_middle_emulation: bool,
    // ── mouse ─────────────────────────────────────────────────────────────────
    pub mouse_natural_scroll: bool,
    #[serde(default)]
    pub mouse_off: bool,
    #[serde(default)]
    pub mouse_accel_speed: f64,
    #[serde(default)]
    pub mouse_accel_profile: AccelProfile,
    #[serde(default)]
    pub mouse_scroll_method: MouseScrollMethod,
    #[serde(default)]
    pub mouse_left_handed: bool,
    #[serde(default)]
    pub mouse_middle_emulation: bool,
    // ── general ──────────────────────────────────────────────────────────────
    pub focus_follows_mouse: bool,
    pub workspace_auto_back_and_forth: bool,
    pub warp_mouse_to_focus: bool,
    pub disable_power_key_handling: bool,
    #[serde(default)]
    pub mod_key: String,
    #[serde(default)]
    pub mod_key_nested: String,
}

/// Settings from the `layout {}` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSettings {
    pub gaps: f64,
    pub center_focused_column: CenterFocusedColumn,
    pub always_center_single_column: bool,
    pub shadow_on: bool,
    pub border_on: bool,
    pub focus_ring_active_color: String,
    pub focus_ring_inactive_color: String,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            gaps: 16.0,
            center_focused_column: CenterFocusedColumn::Never,
            always_center_single_column: false,
            shadow_on: false,
            border_on: false,
            focus_ring_active_color: "#7fc8ff".to_string(),
            focus_ring_inactive_color: "#505050".to_string(),
        }
    }
}

/// Value for the `center-focused-column` layout option.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum CenterFocusedColumn {
    #[default]
    Never,
    Always,
    OnOverflow,
}

impl CenterFocusedColumn {
    pub fn as_kdl_str(&self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Always => "always",
            Self::OnOverflow => "on-overflow",
        }
    }
}

/// Settings from top-level / miscellaneous sections.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MiscSettings {
    // behaviour page
    pub prefer_no_csd: bool,
    pub skip_hotkey_overlay: bool,
    // hotkey-overlay {}
    #[serde(default)]
    pub hotkey_overlay_hide_not_bound: bool,
    // screenshot-path ("": don't override; "null": disable saving)
    #[serde(default)]
    pub screenshot_path: String,
    // cursor {}
    #[serde(default)]
    pub cursor_theme: String,
    #[serde(default)]
    pub cursor_size: u32,                    // 0 = system default
    #[serde(default)]
    pub cursor_hide_when_typing: bool,
    #[serde(default)]
    pub cursor_hide_after_inactive_ms: u32,  // 0 = disabled
    // clipboard {}
    #[serde(default)]
    pub clipboard_disable_primary: bool,
    // overview {}
    #[serde(default)]
    pub overview_zoom: f64,                  // 0.0 = use niri default
    #[serde(default)]
    pub overview_backdrop_color: String,
    // xwayland-satellite {}
    #[serde(default)]
    pub xwayland_off: bool,
    #[serde(default)]
    pub xwayland_path: String,
    // config-notification {}
    #[serde(default)]
    pub config_notification_disable_failed: bool,
}

// ── Animations ────────────────────────────────────────────────────────────────

/// Settings from the `animations {}` block.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimSettings {
    pub global_off: bool,
    #[serde(default)]
    pub slowdown: f64,           // 0.0 = niri default, don't write
    #[serde(default)]
    pub workspace_switch_off: bool,
    #[serde(default)]
    pub window_open_off: bool,
    #[serde(default)]
    pub window_close_off: bool,
    #[serde(default)]
    pub horizontal_view_movement_off: bool,
    #[serde(default)]
    pub window_movement_off: bool,
    #[serde(default)]
    pub window_resize_off: bool,
    #[serde(default)]
    pub config_notification_open_close_off: bool,
    #[serde(default)]
    pub exit_confirmation_open_close_off: bool,
    #[serde(default)]
    pub screenshot_ui_open_off: bool,
    #[serde(default)]
    pub overview_open_close_off: bool,
    #[serde(default)]
    pub recent_windows_close_off: bool,
}

// ── Gestures ──────────────────────────────────────────────────────────────────

/// Settings from the `gestures {}` block.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GestureSettings {
    // dnd-edge-view-scroll (0 = use niri default)
    #[serde(default)]
    pub dnd_view_trigger_width: u32,
    #[serde(default)]
    pub dnd_view_delay_ms: u32,
    #[serde(default)]
    pub dnd_view_max_speed: u32,
    // dnd-edge-workspace-switch (0 = use niri default)
    #[serde(default)]
    pub dnd_ws_trigger_height: u32,
    #[serde(default)]
    pub dnd_ws_delay_ms: u32,
    #[serde(default)]
    pub dnd_ws_max_speed: u32,
    // hot-corners
    #[serde(default)]
    pub hot_corners_off: bool,
    #[serde(default)]
    pub hot_corners_top_left: bool,
    #[serde(default)]
    pub hot_corners_top_right: bool,
    #[serde(default)]
    pub hot_corners_bottom_left: bool,
    #[serde(default)]
    pub hot_corners_bottom_right: bool,
}

// ── Named workspaces ──────────────────────────────────────────────────────────

/// A single named workspace declaration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NamedWorkspace {
    pub name: String,
    #[serde(default)]
    pub open_on_output: String, // empty = no preference
}

// ── Recent windows ────────────────────────────────────────────────────────────

/// Settings from the `recent-windows {}` block.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentWindowsSettings {
    pub off: bool,
    #[serde(default)]
    pub debounce_ms: u32,           // 0 = niri default (750)
    #[serde(default)]
    pub open_delay_ms: u32,         // 0 = niri default (150)
    #[serde(default)]
    pub highlight_active_color: String,
    #[serde(default)]
    pub highlight_urgent_color: String,
    #[serde(default)]
    pub highlight_padding: u32,     // 0 = niri default (30)
    #[serde(default)]
    pub highlight_corner_radius: u32,
    #[serde(default)]
    pub previews_max_height: u32,   // 0 = niri default (480)
    #[serde(default)]
    pub previews_max_scale: f64,    // 0.0 = niri default (0.5)
}

// ── Debug options ─────────────────────────────────────────────────────────────

/// Settings from the `debug {}` block.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugSettings {
    #[serde(default)]
    pub enable_overlay_planes: bool,
    #[serde(default)]
    pub disable_cursor_plane: bool,
    #[serde(default)]
    pub disable_direct_scanout: bool,
    #[serde(default)]
    pub restrict_primary_scanout_to_matching_format: bool,
    #[serde(default)]
    pub force_disable_connectors_on_resume: bool,
    #[serde(default)]
    pub render_drm_device: String,  // empty = don't write
    #[serde(default)]
    pub force_pipewire_invalid_modifier: bool,
    #[serde(default)]
    pub dbus_interfaces_in_non_session_instances: bool,
    #[serde(default)]
    pub wait_for_frame_completion_before_queueing: bool,
    #[serde(default)]
    pub emulate_zero_presentation_time: bool,
    #[serde(default)]
    pub disable_resize_throttling: bool,
    #[serde(default)]
    pub disable_transactions: bool,
    #[serde(default)]
    pub keep_laptop_panel_on_when_lid_is_closed: bool,
    #[serde(default)]
    pub disable_monitor_names: bool,
    #[serde(default)]
    pub strict_new_window_focus_policy: bool,
    #[serde(default)]
    pub honor_xdg_activation_with_invalid_serial: bool,
    #[serde(default)]
    pub skip_cursor_only_updates_during_vrr: bool,
    #[serde(default)]
    pub deactivate_unfocused_windows: bool,
}

// ── Paths ─────────────────────────────────────────────────────────────────────

fn config_base() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config")
        })
}

fn shell_config_dir() -> PathBuf {
    config_base().join("niri-shell")
}

fn json_path() -> PathBuf {
    shell_config_dir().join("settings.json")
}

fn kdl_path() -> PathBuf {
    shell_config_dir().join("settings.kdl")
}

fn niri_config_path() -> PathBuf {
    config_base().join("niri").join("config.kdl")
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Loads settings from disk. Returns defaults if the file does not yet exist.
pub fn load() -> Result<SettingsConfig, SettingsError> {
    let path = json_path();
    if !path.exists() {
        return Ok(SettingsConfig::default());
    }
    let data = std::fs::read_to_string(path)?;
    let mut cfg: SettingsConfig = serde_json::from_str(&data)?;
    // Normalise gaps to an integer value in case a float snuck in from an
    // older version or direct editing.
    cfg.layout.gaps = cfg.layout.gaps.round();
    // Normalise overview zoom to 2 decimal places to avoid IEEE 754 noise in JSON.
    if cfg.misc.overview_zoom > 0.0 {
        cfg.misc.overview_zoom = (cfg.misc.overview_zoom * 20.0).round() / 20.0;
    }
    // Normalise accel speeds to 1 decimal place.
    cfg.input.touchpad_accel_speed =
        (cfg.input.touchpad_accel_speed * 10.0).round() / 10.0;
    cfg.input.mouse_accel_speed =
        (cfg.input.mouse_accel_speed * 10.0).round() / 10.0;
    Ok(cfg)
}

/// Persists settings to disk:
/// 1. Writes `settings.json` (the canonical state).
/// 2. Regenerates `settings.kdl` (the niri include file).
/// 3. Ensures `~/.config/niri/config.kdl` has an include line for it.
pub fn save(cfg: &SettingsConfig) -> Result<(), SettingsError> {
    let dir = shell_config_dir();
    std::fs::create_dir_all(&dir)?;

    // JSON state
    let json = serde_json::to_string_pretty(cfg)?;
    std::fs::write(json_path(), json)?;

    // KDL include file — write atomically so niri never reads a truncated file.
    let kdl = generate_kdl(cfg);
    let final_kdl_path = kdl_path();
    let tmp_kdl_path = final_kdl_path.with_extension("kdl.tmp");
    std::fs::write(&tmp_kdl_path, &kdl)?;
    std::fs::rename(&tmp_kdl_path, &final_kdl_path)?;

    // Wire include line into main niri config
    ensure_include()?;

    Ok(())
}

// ── KDL generation ────────────────────────────────────────────────────────────

/// Generates a valid niri KDL config fragment from typed settings.
/// Every line is produced from a typed field — no freeform string construction.
fn generate_kdl(cfg: &SettingsConfig) -> String {
    let mut out = String::from("// Generated by niri-settings — do not edit manually.\n\n");

    // ── input {} ──────────────────────────────────────────────────────────────
    {
        let i = &cfg.input;
        let mut body = String::new();

        // keyboard sub-block
        {
            let mut kb = String::new();

            // xkb sub-sub-block
            {
                let mut xkb = String::new();
                if !i.keyboard_xkb_layout.is_empty() {
                    xkb.push_str(&format!(
                        "            layout \"{}\"\n",
                        i.keyboard_xkb_layout.replace('"', "\\\"")
                    ));
                }
                if !i.keyboard_xkb_variant.is_empty() {
                    xkb.push_str(&format!(
                        "            variant \"{}\"\n",
                        i.keyboard_xkb_variant.replace('"', "\\\"")
                    ));
                }
                if !i.keyboard_xkb_model.is_empty() {
                    xkb.push_str(&format!(
                        "            model \"{}\"\n",
                        i.keyboard_xkb_model.replace('"', "\\\"")
                    ));
                }
                if !i.keyboard_xkb_rules.is_empty() {
                    xkb.push_str(&format!(
                        "            rules \"{}\"\n",
                        i.keyboard_xkb_rules.replace('"', "\\\"")
                    ));
                }
                if !i.keyboard_xkb_options.is_empty() {
                    xkb.push_str(&format!(
                        "            options \"{}\"\n",
                        i.keyboard_xkb_options.replace('"', "\\\"")
                    ));
                }
                if !xkb.is_empty() {
                    kb.push_str("        xkb {\n");
                    kb.push_str(&xkb);
                    kb.push_str("        }\n");
                }
            }

            if i.keyboard_repeat_delay > 0 {
                kb.push_str(&format!(
                    "        repeat-delay {}\n",
                    i.keyboard_repeat_delay
                ));
            }
            if i.keyboard_repeat_rate > 0 {
                kb.push_str(&format!(
                    "        repeat-rate {}\n",
                    i.keyboard_repeat_rate
                ));
            }
            if i.keyboard_track_layout == TrackLayout::Window {
                kb.push_str("        track-layout \"window\"\n");
            }
            if i.numlock {
                kb.push_str("        numlock\n");
            }
            if !kb.is_empty() {
                body.push_str("    keyboard {\n");
                body.push_str(&kb);
                body.push_str("    }\n");
            }
        }

        // touchpad sub-block
        {
            let mut tp = String::new();
            if i.touchpad_off {
                tp.push_str("        off\n");
            }
            if i.touchpad_tap {
                tp.push_str("        tap\n");
            }
            if i.touchpad_dwt {
                tp.push_str("        dwt\n");
            }
            if i.touchpad_dwtp {
                tp.push_str("        dwtp\n");
            }
            match &i.touchpad_drag {
                DragSetting::Enabled => tp.push_str("        drag true\n"),
                DragSetting::Disabled => tp.push_str("        drag false\n"),
                DragSetting::Default => {}
            }
            if i.touchpad_drag_lock {
                tp.push_str("        drag-lock\n");
            }
            if i.touchpad_natural_scroll {
                tp.push_str("        natural-scroll\n");
            }
            if i.touchpad_accel_speed != 0.0 {
                tp.push_str(&format!(
                    "        accel-speed {:.1}\n",
                    i.touchpad_accel_speed
                ));
            }
            if let Some(s) = i.touchpad_accel_profile.as_kdl_str() {
                tp.push_str(&format!("        accel-profile \"{s}\"\n"));
            }
            if let Some(s) = i.touchpad_scroll_method.as_kdl_str() {
                tp.push_str(&format!("        scroll-method \"{s}\"\n"));
            }
            if let Some(s) = i.touchpad_tap_button_map.as_kdl_str() {
                tp.push_str(&format!("        tap-button-map \"{s}\"\n"));
            }
            if let Some(s) = i.touchpad_click_method.as_kdl_str() {
                tp.push_str(&format!("        click-method \"{s}\"\n"));
            }
            if i.touchpad_disabled_on_external_mouse {
                tp.push_str("        disabled-on-external-mouse\n");
            }
            if i.touchpad_left_handed {
                tp.push_str("        left-handed\n");
            }
            if i.touchpad_middle_emulation {
                tp.push_str("        middle-emulation\n");
            }
            if !tp.is_empty() {
                body.push_str("    touchpad {\n");
                body.push_str(&tp);
                body.push_str("    }\n");
            }
        }

        // mouse sub-block
        {
            let mut ms = String::new();
            if i.mouse_off {
                ms.push_str("        off\n");
            }
            if i.mouse_natural_scroll {
                ms.push_str("        natural-scroll\n");
            }
            if i.mouse_accel_speed != 0.0 {
                ms.push_str(&format!("        accel-speed {:.1}\n", i.mouse_accel_speed));
            }
            if let Some(s) = i.mouse_accel_profile.as_kdl_str() {
                ms.push_str(&format!("        accel-profile \"{s}\"\n"));
            }
            if let Some(s) = i.mouse_scroll_method.as_kdl_str() {
                ms.push_str(&format!("        scroll-method \"{s}\"\n"));
            }
            if i.mouse_left_handed {
                ms.push_str("        left-handed\n");
            }
            if i.mouse_middle_emulation {
                ms.push_str("        middle-emulation\n");
            }
            if !ms.is_empty() {
                body.push_str("    mouse {\n");
                body.push_str(&ms);
                body.push_str("    }\n");
            }
        }

        // general flags
        if i.focus_follows_mouse {
            body.push_str("    focus-follows-mouse\n");
        }
        if i.workspace_auto_back_and_forth {
            body.push_str("    workspace-auto-back-and-forth\n");
        }
        if i.warp_mouse_to_focus {
            body.push_str("    warp-mouse-to-focus\n");
        }
        if i.disable_power_key_handling {
            body.push_str("    disable-power-key-handling\n");
        }
        if !i.mod_key.is_empty() {
            body.push_str(&format!(
                "    mod-key \"{}\"\n",
                i.mod_key.replace('"', "\\\"")
            ));
        }
        if !i.mod_key_nested.is_empty() {
            body.push_str(&format!(
                "    mod-key-nested \"{}\"\n",
                i.mod_key_nested.replace('"', "\\\"")
            ));
        }

        if !body.is_empty() {
            out.push_str("input {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    // ── layout {} ─────────────────────────────────────────────────────────────
    {
        let l = &cfg.layout;
        out.push_str("layout {\n");
        out.push_str(&format!("    gaps {}\n", l.gaps.round() as i64));
        out.push_str(&format!(
            "    center-focused-column \"{}\"\n",
            l.center_focused_column.as_kdl_str()
        ));
        if l.always_center_single_column {
            out.push_str("    always-center-single-column\n");
        }

        // focus-ring (always written so we can set colors)
        out.push_str("    focus-ring {\n");
        out.push_str(&format!(
            "        active-color \"{}\"\n",
            sanitise_color(&l.focus_ring_active_color)
        ));
        out.push_str(&format!(
            "        inactive-color \"{}\"\n",
            sanitise_color(&l.focus_ring_inactive_color)
        ));
        out.push_str("    }\n");

        if l.border_on {
            out.push_str("    border {\n        on\n    }\n");
        }
        if l.shadow_on {
            out.push_str("    shadow {\n        on\n    }\n");
        }

        out.push_str("}\n\n");
    }

    // ── top-level misc ────────────────────────────────────────────────────────
    {
        let m = &cfg.misc;

        if m.prefer_no_csd {
            out.push_str("prefer-no-csd\n\n");
        }

        // screenshot-path
        // Normalise "null" case-insensitively so "NULL" / "Null" also disables saving.
        let shot_path = m.screenshot_path.trim();
        if shot_path.eq_ignore_ascii_case("null") {
            out.push_str("screenshot-path null\n\n");
        } else if !shot_path.is_empty() {
            out.push_str(&format!(
                "screenshot-path \"{}\"\n\n",
                sanitise_path(shot_path)
            ));
        }

        // cursor {}
        {
            let mut body = String::new();
            if !m.cursor_theme.is_empty() {
                body.push_str(&format!(
                    "    xcursor-theme \"{}\"\n",
                    sanitise_path(&m.cursor_theme)
                ));
            }
            if m.cursor_size > 0 {
                body.push_str(&format!("    xcursor-size {}\n", m.cursor_size.min(512)));
            }
            if m.cursor_hide_when_typing {
                body.push_str("    hide-when-typing\n");
            }
            if m.cursor_hide_after_inactive_ms > 0 {
                body.push_str(&format!(
                    "    hide-after-inactive-ms {}\n",
                    m.cursor_hide_after_inactive_ms
                ));
            }
            if !body.is_empty() {
                out.push_str("cursor {\n");
                out.push_str(&body);
                out.push_str("}\n\n");
            }
        }

        // overview {}
        {
            let mut body = String::new();
            if m.overview_zoom > 0.0 {
                body.push_str(&format!(
                    "    zoom {:.2}\n",
                    m.overview_zoom.clamp(0.01, 0.75)
                ));
            }
            // Only write backdrop-color if the value is a valid hex color.
            // An invalid / partially-typed value is silently skipped to avoid
            // writing a wrong fallback color (e.g. #ffffff) into the config.
            if is_valid_color(&m.overview_backdrop_color) {
                body.push_str(&format!(
                    "    backdrop-color \"{}\"\n",
                    sanitise_color(&m.overview_backdrop_color)
                ));
            }
            if !body.is_empty() {
                out.push_str("overview {\n");
                out.push_str(&body);
                out.push_str("}\n\n");
            }
        }

        // clipboard {}
        if m.clipboard_disable_primary {
            out.push_str("clipboard {\n    disable-primary\n}\n\n");
        }

        // hotkey-overlay {}
        {
            let mut body = String::new();
            if m.skip_hotkey_overlay {
                body.push_str("    skip-at-startup\n");
            }
            if m.hotkey_overlay_hide_not_bound {
                body.push_str("    hide-not-bound\n");
            }
            if !body.is_empty() {
                out.push_str("hotkey-overlay {\n");
                out.push_str(&body);
                out.push_str("}\n\n");
            }
        }

        // config-notification {}
        if m.config_notification_disable_failed {
            out.push_str("config-notification {\n    disable-failed\n}\n\n");
        }

        // xwayland-satellite {}
        {
            let mut body = String::new();
            if m.xwayland_off {
                body.push_str("    off\n");
            } else if !m.xwayland_path.is_empty() {
                body.push_str(&format!(
                    "    path \"{}\"\n",
                    sanitise_path(&m.xwayland_path)
                ));
            }
            if !body.is_empty() {
                out.push_str("xwayland-satellite {\n");
                out.push_str(&body);
                out.push_str("}\n\n");
            }
        }
    }

    // ── animations {} ─────────────────────────────────────────────────────────
    {
        let a = &cfg.anim;
        if a.global_off {
            out.push_str("animations {\n    off\n}\n\n");
        } else {
            let mut body = String::new();
            if a.slowdown != 0.0 {
                body.push_str(&format!("    slowdown {:.1}\n", a.slowdown));
            }
            let anim_flags: &[(&str, bool)] = &[
                ("workspace-switch",                a.workspace_switch_off),
                ("window-open",                     a.window_open_off),
                ("window-close",                    a.window_close_off),
                ("horizontal-view-movement",        a.horizontal_view_movement_off),
                ("window-movement",                 a.window_movement_off),
                ("window-resize",                   a.window_resize_off),
                ("config-notification-open-close",  a.config_notification_open_close_off),
                ("exit-confirmation-open-close",    a.exit_confirmation_open_close_off),
                ("screenshot-ui-open",              a.screenshot_ui_open_off),
                ("overview-open-close",             a.overview_open_close_off),
                ("recent-windows-close",            a.recent_windows_close_off),
            ];
            for (name, is_off) in anim_flags {
                if *is_off {
                    body.push_str(&format!("    {name} {{\n        off\n    }}\n"));
                }
            }
            if !body.is_empty() {
                out.push_str("animations {\n");
                out.push_str(&body);
                out.push_str("}\n\n");
            }
        }
    }

    // ── workspace "name" {} entries ────────────────────────────────────────────
    for ws in &cfg.workspaces {
        let name = ws.name.replace('"', "\\\"");
        if ws.open_on_output.is_empty() {
            out.push_str(&format!("workspace \"{name}\"\n"));
        } else {
            let out_name = ws.open_on_output.replace('"', "\\\"");
            out.push_str(&format!(
                "workspace \"{name}\" {{\n    open-on-output \"{out_name}\"\n}}\n"
            ));
        }
    }
    if !cfg.workspaces.is_empty() {
        out.push('\n');
    }

    // ── gestures {} ────────────────────────────────────────────────────────────
    {
        let g = &cfg.gestures;
        let mut body = String::new();

        if g.dnd_view_trigger_width != 0 || g.dnd_view_delay_ms != 0 || g.dnd_view_max_speed != 0 {
            body.push_str("    dnd-edge-view-scroll {\n");
            if g.dnd_view_trigger_width != 0 {
                body.push_str(&format!("        trigger-width {}\n", g.dnd_view_trigger_width));
            }
            if g.dnd_view_delay_ms != 0 {
                body.push_str(&format!("        delay-ms {}\n", g.dnd_view_delay_ms));
            }
            if g.dnd_view_max_speed != 0 {
                body.push_str(&format!("        max-speed {}\n", g.dnd_view_max_speed));
            }
            body.push_str("    }\n");
        }

        if g.dnd_ws_trigger_height != 0 || g.dnd_ws_delay_ms != 0 || g.dnd_ws_max_speed != 0 {
            body.push_str("    dnd-edge-workspace-switch {\n");
            if g.dnd_ws_trigger_height != 0 {
                body.push_str(&format!("        trigger-height {}\n", g.dnd_ws_trigger_height));
            }
            if g.dnd_ws_delay_ms != 0 {
                body.push_str(&format!("        delay-ms {}\n", g.dnd_ws_delay_ms));
            }
            if g.dnd_ws_max_speed != 0 {
                body.push_str(&format!("        max-speed {}\n", g.dnd_ws_max_speed));
            }
            body.push_str("    }\n");
        }

        if g.hot_corners_off {
            body.push_str("    hot-corners {\n        off\n    }\n");
        } else if g.hot_corners_top_left
            || g.hot_corners_top_right
            || g.hot_corners_bottom_left
            || g.hot_corners_bottom_right
        {
            let mut hc = String::new();
            if g.hot_corners_top_left {
                hc.push_str("        top-left\n");
            }
            if g.hot_corners_top_right {
                hc.push_str("        top-right\n");
            }
            if g.hot_corners_bottom_left {
                hc.push_str("        bottom-left\n");
            }
            if g.hot_corners_bottom_right {
                hc.push_str("        bottom-right\n");
            }
            body.push_str("    hot-corners {\n");
            body.push_str(&hc);
            body.push_str("    }\n");
        }

        if !body.is_empty() {
            out.push_str("gestures {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    // ── recent-windows {} ──────────────────────────────────────────────────────
    {
        let r = &cfg.recent_windows;
        if r.off {
            out.push_str("recent-windows {\n    off\n}\n\n");
        } else {
            let mut body = String::new();
            if r.debounce_ms != 0 {
                body.push_str(&format!("    debounce-ms {}\n", r.debounce_ms));
            }
            if r.open_delay_ms != 0 {
                body.push_str(&format!("    open-delay-ms {}\n", r.open_delay_ms));
            }

            // highlight sub-block
            {
                let mut hl = String::new();
                if is_valid_color(&r.highlight_active_color) {
                    hl.push_str(&format!(
                        "        active-color \"{}\"\n",
                        sanitise_color(&r.highlight_active_color)
                    ));
                }
                if is_valid_color(&r.highlight_urgent_color) {
                    hl.push_str(&format!(
                        "        urgent-color \"{}\"\n",
                        sanitise_color(&r.highlight_urgent_color)
                    ));
                }
                if r.highlight_padding != 0 {
                    hl.push_str(&format!("        padding {}\n", r.highlight_padding));
                }
                if r.highlight_corner_radius != 0 {
                    hl.push_str(&format!(
                        "        corner-radius {}\n",
                        r.highlight_corner_radius
                    ));
                }
                if !hl.is_empty() {
                    body.push_str("    highlight {\n");
                    body.push_str(&hl);
                    body.push_str("    }\n");
                }
            }

            // previews sub-block
            {
                let mut pv = String::new();
                if r.previews_max_height != 0 {
                    pv.push_str(&format!("        max-height {}\n", r.previews_max_height));
                }
                if r.previews_max_scale != 0.0 {
                    pv.push_str(&format!("        max-scale {:.2}\n", r.previews_max_scale));
                }
                if !pv.is_empty() {
                    body.push_str("    previews {\n");
                    body.push_str(&pv);
                    body.push_str("    }\n");
                }
            }

            if !body.is_empty() {
                out.push_str("recent-windows {\n");
                out.push_str(&body);
                out.push_str("}\n\n");
            }
        }
    }

    // ── debug {} ───────────────────────────────────────────────────────────────
    {
        let d = &cfg.debug;
        let mut body = String::new();

        let flags: &[(&str, bool)] = &[
            ("enable-overlay-planes",                       d.enable_overlay_planes),
            ("disable-cursor-plane",                        d.disable_cursor_plane),
            ("disable-direct-scanout",                      d.disable_direct_scanout),
            ("restrict-primary-scanout-to-matching-format", d.restrict_primary_scanout_to_matching_format),
            ("force-disable-connectors-on-resume",          d.force_disable_connectors_on_resume),
            ("force-pipewire-invalid-modifier",             d.force_pipewire_invalid_modifier),
            ("dbus-interfaces-in-non-session-instances",    d.dbus_interfaces_in_non_session_instances),
            ("wait-for-frame-completion-before-queueing",   d.wait_for_frame_completion_before_queueing),
            ("emulate-zero-presentation-time",              d.emulate_zero_presentation_time),
            ("disable-resize-throttling",                   d.disable_resize_throttling),
            ("disable-transactions",                        d.disable_transactions),
            ("keep-laptop-panel-on-when-lid-is-closed",     d.keep_laptop_panel_on_when_lid_is_closed),
            ("disable-monitor-names",                       d.disable_monitor_names),
            ("strict-new-window-focus-policy",              d.strict_new_window_focus_policy),
            ("honor-xdg-activation-with-invalid-serial",    d.honor_xdg_activation_with_invalid_serial),
            ("skip-cursor-only-updates-during-vrr",         d.skip_cursor_only_updates_during_vrr),
            ("deactivate-unfocused-windows",                d.deactivate_unfocused_windows),
        ];
        for (name, enabled) in flags {
            if *enabled {
                body.push_str(&format!("    {name}\n"));
            }
        }
        if !d.render_drm_device.is_empty() {
            body.push_str(&format!(
                "    render-drm-device \"{}\"\n",
                sanitise_path(&d.render_drm_device)
            ));
        }

        if !body.is_empty() {
            out.push_str("debug {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    // ── output "name" {} blocks ────────────────────────────────────────────────
    for o in &cfg.outputs {
        if o.name.is_empty() {
            continue;
        }
        let quoted = format!("\"{}\"", o.name.replace('"', "\\\""));
        let mut body = String::new();

        if o.off {
            body.push_str("    off\n");
        } else {
            // mode
            if o.mode_width != 0 && o.mode_height != 0 {
                if o.mode_refresh_mhz != 0 {
                    let hz = o.mode_refresh_mhz as f64 / 1000.0;
                    body.push_str(&format!(
                        "    mode \"{}x{}@{:.3}\"\n",
                        o.mode_width, o.mode_height, hz
                    ));
                } else {
                    body.push_str(&format!("    mode \"{}x{}\"\n", o.mode_width, o.mode_height));
                }
            }
            // scale
            if o.scale != 0.0 {
                body.push_str(&format!("    scale {:.2}\n", o.scale));
            }
            // transform
            if o.transform != OutputTransform::Normal {
                body.push_str(&format!("    transform \"{}\"\n", o.transform.as_kdl_str()));
            }
            // position
            if o.position_set {
                body.push_str(&format!("    position x={} y={}\n", o.position_x, o.position_y));
            }
            // vrr
            if o.vrr != VrrMode::Default {
                body.push_str(&format!("    vrr {}\n", o.vrr.as_kdl_str()));
            }
        }

        out.push_str(&format!("output {quoted} {{\n"));
        out.push_str(&body);
        out.push_str("}\n\n");
    }

    // ── window-rule {} blocks ──────────────────────────────────────────────────
    for r in &cfg.window_rules {
        let mut body = String::new();

        // match criteria
        if !r.match_app_id.is_empty() {
            body.push_str(&format!(
                "    match app-id=\"{}\"\n",
                sanitise_regex(&r.match_app_id)
            ));
        }
        if !r.match_title.is_empty() {
            body.push_str(&format!(
                "    match title=\"{}\"\n",
                sanitise_regex(&r.match_title)
            ));
        }
        match &r.match_at_startup {
            TriState::On  => body.push_str("    match at-startup=true\n"),
            TriState::Off => body.push_str("    match at-startup=false\n"),
            TriState::Default => {}
        }

        // opening properties
        match &r.open_maximized {
            TriState::On  => body.push_str("    open-maximized true\n"),
            TriState::Off => body.push_str("    open-maximized false\n"),
            TriState::Default => {}
        }
        match &r.open_fullscreen {
            TriState::On  => body.push_str("    open-fullscreen true\n"),
            TriState::Off => body.push_str("    open-fullscreen false\n"),
            TriState::Default => {}
        }
        match &r.open_floating {
            TriState::On  => body.push_str("    open-floating true\n"),
            TriState::Off => body.push_str("    open-floating false\n"),
            TriState::Default => {}
        }
        match &r.open_focused {
            TriState::On  => body.push_str("    open-focused true\n"),
            TriState::Off => body.push_str("    open-focused false\n"),
            TriState::Default => {}
        }
        if !r.open_on_output.is_empty() {
            body.push_str(&format!(
                "    open-on-output \"{}\"\n",
                sanitise_path(&r.open_on_output)
            ));
        }
        if !r.open_on_workspace.is_empty() {
            body.push_str(&format!(
                "    open-on-workspace \"{}\"\n",
                sanitise_path(&r.open_on_workspace)
            ));
        }

        // dynamic properties
        if r.opacity != 0.0 {
            body.push_str(&format!("    opacity {:.2}\n", r.opacity));
        }
        if r.block_out_from != BlockOutFrom::None {
            body.push_str(&format!(
                "    block-out-from \"{}\"\n",
                r.block_out_from.as_kdl_str()
            ));
        }
        match &r.draw_border_with_background {
            TriState::On  => body.push_str("    draw-border-with-background true\n"),
            TriState::Off => body.push_str("    draw-border-with-background false\n"),
            TriState::Default => {}
        }
        if r.geometry_corner_radius != 0.0 {
            body.push_str(&format!("    geometry-corner-radius {:.1}\n", r.geometry_corner_radius));
        }
        match &r.clip_to_geometry {
            TriState::On  => body.push_str("    clip-to-geometry true\n"),
            TriState::Off => body.push_str("    clip-to-geometry false\n"),
            TriState::Default => {}
        }
        match &r.variable_refresh_rate {
            TriState::On  => body.push_str("    variable-refresh-rate true\n"),
            TriState::Off => body.push_str("    variable-refresh-rate false\n"),
            TriState::Default => {}
        }
        if r.min_width  != 0 { body.push_str(&format!("    min-width {}\n",  r.min_width));  }
        if r.max_width  != 0 { body.push_str(&format!("    max-width {}\n",  r.max_width));  }
        if r.min_height != 0 { body.push_str(&format!("    min-height {}\n", r.min_height)); }
        if r.max_height != 0 { body.push_str(&format!("    max-height {}\n", r.max_height)); }
        if r.scroll_factor != 0.0 {
            body.push_str(&format!("    scroll-factor {:.2}\n", r.scroll_factor));
        }

        if !body.is_empty() {
            out.push_str("window-rule {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    // ── layer-rule {} blocks ───────────────────────────────────────────────────
    for r in &cfg.layer_rules {
        let mut body = String::new();

        if !r.match_namespace.is_empty() {
            body.push_str(&format!(
                "    match namespace=\"{}\"\n",
                sanitise_regex(&r.match_namespace)
            ));
        }
        match &r.match_at_startup {
            TriState::On  => body.push_str("    match at-startup=true\n"),
            TriState::Off => body.push_str("    match at-startup=false\n"),
            TriState::Default => {}
        }
        if r.opacity != 0.0 {
            body.push_str(&format!("    opacity {:.2}\n", r.opacity));
        }
        if r.block_out_from != BlockOutFrom::None {
            body.push_str(&format!(
                "    block-out-from \"{}\"\n",
                r.block_out_from.as_kdl_str()
            ));
        }
        match &r.shadow {
            TriState::On  => body.push_str("    shadow {\n        on\n    }\n"),
            TriState::Off => body.push_str("    shadow {\n        off\n    }\n"),
            TriState::Default => {}
        }
        if r.geometry_corner_radius != 0.0 {
            body.push_str(&format!("    geometry-corner-radius {:.1}\n", r.geometry_corner_radius));
        }
        match &r.place_within_backdrop {
            TriState::On  => body.push_str("    place-within-backdrop true\n"),
            TriState::Off => body.push_str("    place-within-backdrop false\n"),
            TriState::Default => {}
        }

        if !body.is_empty() {
            out.push_str("layer-rule {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    // ── switch-events {} ───────────────────────────────────────────────────────
    {
        let se = &cfg.switch_events;
        let events: &[(&str, &Vec<String>)] = &[
            ("lid-close",      &se.lid_close),
            ("lid-open",       &se.lid_open),
            ("tablet-mode-on", &se.tablet_mode_on),
            ("tablet-mode-off",&se.tablet_mode_off),
        ];
        let mut body = String::new();
        for (name, argv) in events {
            if argv.is_empty() { continue; }
            let args = argv
                .iter()
                .map(|a| format!("\"{}\"", sanitise_path(a)))
                .collect::<Vec<_>>()
                .join(" ");
            body.push_str(&format!("    {name} {{ spawn {args}; }}\n"));
        }
        if !body.is_empty() {
            out.push_str("switch-events {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    // ── binds {} ───────────────────────────────────────────────────────────────
    {
        let mut body = String::new();
        for b in &cfg.binds {
            if b.key.is_empty() || b.action.is_empty() {
                continue;
            }
            let key = sanitise_keybind_key(&b.key);
            if key.is_empty() {
                continue;
            }

            let mut props = String::new();
            if !b.repeat {
                props.push_str(" repeat=false");
            }
            if b.cooldown_ms > 0 {
                props.push_str(&format!(" cooldown-ms={}", b.cooldown_ms));
            }
            if b.allow_when_locked {
                props.push_str(" allow-when-locked=true");
            }

            let action_kdl = match b.action.as_str() {
                "spawn" => {
                    let args: Vec<String> = b
                        .action_args
                        .split_whitespace()
                        .map(|a| format!("\"{}\"", sanitise_path(a)))
                        .collect();
                    if args.is_empty() {
                        continue;
                    }
                    format!("spawn {}", args.join(" "))
                }
                "spawn-sh" => {
                    if b.action_args.is_empty() {
                        continue;
                    }
                    format!("spawn-sh \"{}\"", sanitise_path(&b.action_args))
                }
                name if !b.action_args.is_empty() => {
                    format!("{} {}", name, sanitise_path(&b.action_args))
                }
                name => name.to_string(),
            };

            body.push_str(&format!("    {key}{props} {{ {action_kdl}; }}\n"));
        }
        if !body.is_empty() {
            out.push_str("binds {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    out
}

/// Validates that a colour string is a safe `#rrggbb` / `#rrggbbaa` hex literal.
/// Returns the unchanged string if valid, or a safe white fallback otherwise.
/// This prevents any injection into the generated KDL file.
fn sanitise_color(s: &str) -> String {
    let s = s.trim();
    let valid = s.starts_with('#')
        && matches!(s.len(), 4 | 5 | 7 | 9) // #rgb #rgba #rrggbb #rrggbbaa
        && s[1..].chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        s.to_string()
    } else {
        "#ffffff".to_string()
    }
}

/// Returns `true` when `s` is a valid `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa` string.
fn is_valid_color(s: &str) -> bool {
    let s = s.trim();
    s.starts_with('#')
        && matches!(s.len(), 4 | 5 | 7 | 9)
        && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Strips characters that could break a KDL string literal (double-quote,
/// backslash, and control characters). Safe to embed inside `"..."` in KDL.
fn sanitise_path(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '"' && *c != '\\' && !c.is_control())
        .collect()
}

/// Sanitises a regex string for safe embedding in a KDL string literal.
/// Allows the regex metacharacters niri accepts, strips literal `"` and
/// control characters which would break KDL parsing.
fn sanitise_regex(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '"' && !c.is_control())
        .collect()
}

/// Strips characters not valid in a niri key-combo string.
/// Allows ASCII alphanumerics, `+`, `-`, and `_` (e.g. `ISO_Level3_Shift`).
/// This prevents injection into the `binds { key { action; } }` KDL block.
fn sanitise_keybind_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '_'))
        .collect()
}

/// Parse a single bind line from niri config KDL.
///
/// Expected form (all on one line):
/// ```text
/// KeyCombo [prop=val …] { action [args]; }
/// ```
///
/// Returns `None` for comments, blank lines, or lines that can't be parsed.
fn parse_bind_line(line: &str) -> Option<Keybind> {
    let trimmed = line.trim();
    // Skip comments and empty lines.
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }
    // Must contain both `{` and `}` on the same line.
    let brace_open = trimmed.find('{')?;
    let brace_close = trimmed.rfind('}')?;
    if brace_close <= brace_open {
        return None;
    }

    // Everything before the opening `{` is the hotkey + properties portion.
    let before_brace = trimmed[..brace_open].trim();
    // Everything between `{` and `}` is `action [args];` — strip the trailing `;`.
    let action_part = trimmed[brace_open + 1..brace_close]
        .trim()
        .trim_end_matches(';')
        .trim();

    if action_part.is_empty() || before_brace.is_empty() {
        return None;
    }

    // Split the hotkey portion into tokens.
    let mut tokens = before_brace.split_whitespace();
    let key = tokens.next()?.to_string();
    if key.is_empty() || key.contains('"') {
        return None;
    }

    // Parse optional bind properties from the remaining tokens.
    let mut repeat = true;
    let mut cooldown_ms: u32 = 0;
    let mut allow_when_locked = false;
    for tok in tokens {
        if tok == "repeat=false" {
            repeat = false;
        } else if let Some(ms) = tok.strip_prefix("cooldown-ms=") {
            match ms.parse() {
                Ok(val) => cooldown_ms = val,
                Err(_) => {
                    log::warn!("settings: invalid cooldown-ms value '{ms}' in bind: {line}");
                    return None;
                }
            }
        } else if tok == "allow-when-locked=true" {
            allow_when_locked = true;
        }
        // Ignore hotkey-overlay-title, allow-inhibiting, write-to-disk, etc.
    }

    // Parse the action and its arguments.
    // Actions use kebab-case; arguments are either bare words or `"…"` strings.
    let (action, action_args) = parse_action_and_args(action_part);

    Some(Keybind { key, action, action_args, repeat, cooldown_ms, allow_when_locked })
}

/// Split `action [arg1 arg2 …]` into the action name and a single
/// space-joined argument string.  Quoted strings like `"foo bar"` are kept
/// intact (without their surrounding quotes).
fn parse_action_and_args(s: &str) -> (String, String) {
    let s = s.trim();
    // Find where the action name ends (first whitespace).
    let split = s.find(|c: char| c.is_whitespace()).unwrap_or(s.len());
    let action = s[..split].to_string();
    let rest = s[split..].trim();
    if rest.is_empty() {
        return (action, String::new());
    }

    // Collect argument tokens — unquote `"arg"` values.
    let mut args: Vec<String> = Vec::new();
    let mut chars = rest.chars().peekable();
    while chars.peek().is_some() {
        // Skip whitespace between tokens.
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }
        match chars.peek() {
            None => break,
            Some('"') => {
                chars.next(); // consume opening `"`
                let mut buf = String::new();
                for c in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                    buf.push(c);
                }
                if !buf.is_empty() {
                    args.push(buf);
                }
            }
            _ => {
                let mut buf = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    }
                    chars.next();
                    buf.push(c);
                }
                if !buf.is_empty() {
                    args.push(buf);
                }
            }
        }
    }

    let joined = args.join(" ");
    (action, joined)
}

/// Read `~/.config/niri/config.kdl` and extract all bind entries from the
/// first `binds { … }` block.  Skips commented-out lines.
///
/// Returns an empty `Vec` if the config cannot be read or contains no binds.
pub fn import_binds_from_niri_config() -> Vec<Keybind> {
    let path = niri_config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("settings: could not read niri config for bind import: {e}");
            return Vec::new();
        }
    };

    let mut binds = Vec::new();
    let mut in_binds = false;
    let mut depth: u32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        if !in_binds {
            // Detect `binds {` — the block opener.
            if trimmed.starts_with("binds {") {
                in_binds = true;
                depth = 1;
            }
            continue;
        }

        // Track nested braces so we exit on the correct `}`.
        let opens = trimmed.chars().filter(|&c| c == '{').count() as u32;
        let closes = trimmed.chars().filter(|&c| c == '}').count() as u32;

        // A line that is just `}` closes our block.
        if trimmed == "}" {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                break;
            }
            continue;
        }

        depth = depth.saturating_add(opens).saturating_sub(closes);

        if let Some(bind) = parse_bind_line(trimmed) {
            binds.push(bind);
        }
    }

    log::info!("settings: imported {} binds from niri config", binds.len());
    binds
}

/// Adds `include "~/.config/niri-shell/settings.kdl"` to the main niri config
/// if it is not already present.
fn ensure_include() -> Result<(), SettingsError> {
    let config_path = niri_config_path();
    if !config_path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&config_path)?;
    // Use the absolute path — niri's include does not expand `~`.
    let abs_kdl = kdl_path();
    let include_line = format!(r#"include "{}""#, abs_kdl.to_string_lossy());
    if content.contains(&include_line) {
        return Ok(());
    }
    // Also remove any old `~`-based line that may have been written by a
    // previous version, to avoid duplicate / broken includes.
    let cleaned = content
        .lines()
        .filter(|l| !l.contains(r#"include "~/.config/niri-shell/settings.kdl""#))
        .collect::<Vec<_>>()
        .join("\n");
    let new_content = format!(
        "{}\n\n// Added by niri-settings — overrides settings above\n{}\n",
        cleaned.trim_end(),
        include_line
    );
    std::fs::write(&config_path, new_content)?;
    log::info!("settings: added include line to niri config");
    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trips() {
        let cfg = SettingsConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: SettingsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.layout.gaps, cfg.layout.gaps);
    }

    #[test]
    fn kdl_contains_gaps() {
        let mut cfg = SettingsConfig::default();
        cfg.layout.gaps = 24.0;
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("gaps 24"), "expected 'gaps 24' in: {kdl}");
    }

    #[test]
    fn kdl_touchpad_tap() {
        let mut cfg = SettingsConfig::default();
        cfg.input.touchpad_tap = true;
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("tap"), "expected 'tap' in: {kdl}");
    }

    #[test]
    fn kdl_prefer_no_csd() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.prefer_no_csd = true;
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("prefer-no-csd"), "expected prefer-no-csd: {kdl}");
    }

    #[test]
    fn sanitise_color_strips_junk() {
        // valid colors pass through unchanged
        assert_eq!(sanitise_color("#7fc8ff"), "#7fc8ff");
        assert_eq!(sanitise_color("#fff"), "#fff");
        assert_eq!(sanitise_color("#7fc8ff80"), "#7fc8ff80");
        // invalid / injected strings get a safe default
        assert_eq!(sanitise_color("#7fc8ff; DROP TABLE"), "#ffffff");
        assert_eq!(sanitise_color("red"), "#ffffff");
        assert_eq!(sanitise_color(""), "#ffffff");
    }

    #[test]
    fn center_focused_column_variants() {
        assert_eq!(CenterFocusedColumn::Never.as_kdl_str(), "never");
        assert_eq!(CenterFocusedColumn::Always.as_kdl_str(), "always");
        assert_eq!(CenterFocusedColumn::OnOverflow.as_kdl_str(), "on-overflow");
    }

    // ── Miscellaneous section ─────────────────────────────────────────────────

    #[test]
    fn screenshot_path_null_case_insensitive() {
        // Bug: "NULL" and "Null" must emit the KDL null keyword, not a quoted path.
        for variant in &["null", "NULL", "Null", "nUlL"] {
            let mut cfg = SettingsConfig::default();
            cfg.misc.screenshot_path = variant.to_string();
            let kdl = generate_kdl(&cfg);
            assert!(
                kdl.contains("screenshot-path null\n"),
                "'{}' should produce unquoted null but got:\n{kdl}",
                variant
            );
            assert!(
                !kdl.contains("screenshot-path \""),
                "'{}' must not produce a quoted path: {kdl}",
                variant
            );
        }
    }

    #[test]
    fn screenshot_path_regular_value() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.screenshot_path = "~/Pictures/Screenshots/%Y-%m-%d.png".to_string();
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("screenshot-path \"~/Pictures/Screenshots/"),
            "expected quoted path in: {kdl}"
        );
    }

    #[test]
    fn overview_backdrop_invalid_color_skipped() {
        // Bug: partial color (wrong length) must not write a fallback #ffffff.
        let mut cfg = SettingsConfig::default();
        cfg.misc.overview_backdrop_color = "#26262".to_string(); // 6 chars — invalid
        let kdl = generate_kdl(&cfg);
        assert!(
            !kdl.contains("backdrop-color"),
            "invalid color should be omitted entirely, got:\n{kdl}"
        );
    }

    #[test]
    fn overview_backdrop_valid_color_written() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.overview_backdrop_color = "#262626".to_string();
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("backdrop-color \"#262626\""), "expected backdrop-color: {kdl}");
    }

    #[test]
    fn overview_zoom_float_normalised_on_load() {
        // Simulate a noisy float that could result from IEEE 754 arithmetic
        // when slider rounding produces e.g. 0.30000000000000004.
        let mut cfg = SettingsConfig::default();
        cfg.misc.overview_zoom = 0.300_000_000_000_000_04_f64;
        // Apply the same normalisation that load() does.
        if cfg.misc.overview_zoom > 0.0 {
            cfg.misc.overview_zoom = (cfg.misc.overview_zoom * 20.0).round() / 20.0;
        }
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("zoom 0.30"), "expected zoom 0.30 but got:\n{kdl}");
    }

    #[test]
    fn cursor_block_only_written_when_non_default() {
        let cfg = SettingsConfig::default();
        let kdl = generate_kdl(&cfg);
        assert!(!kdl.contains("cursor {"), "cursor block should be absent for default config");
    }

    #[test]
    fn xwayland_off_takes_priority_over_path() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.xwayland_off = true;
        cfg.misc.xwayland_path = "/usr/local/bin/xwayland-satellite".to_string();
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("xwayland-satellite {\n    off"), "expected off block: {kdl}");
        assert!(!kdl.contains("path"), "path should be suppressed when off is set: {kdl}");
    }

    #[test]
    fn switch_events_written_when_set() {
        let mut cfg = SettingsConfig::default();
        cfg.switch_events.lid_close = vec!["notify-send".into(), "Lid closed".into()];
        cfg.switch_events.tablet_mode_on = vec!["wvkbd-mobintl".into()];
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("switch-events {"), "expected switch-events block");
        assert!(kdl.contains(r#"lid-close { spawn "notify-send" "Lid closed"; }"#));
        assert!(kdl.contains(r#"tablet-mode-on { spawn "wvkbd-mobintl"; }"#));
        assert!(!kdl.contains("lid-open"), "lid-open should be absent when empty");
    }

    #[test]
    fn switch_events_absent_for_default() {
        let cfg = SettingsConfig::default();
        let kdl = generate_kdl(&cfg);
        assert!(!kdl.contains("switch-events"), "switch-events absent when all empty");
    }

    #[test]
    fn window_rule_written_with_match_and_dynamic_props() {
        let mut cfg = SettingsConfig::default();
        let mut r = WindowRule::default();
        r.match_app_id = "^firefox$".into();
        r.open_floating = TriState::On;
        r.opacity = 0.85;
        r.block_out_from = BlockOutFrom::Screencast;
        cfg.window_rules.push(r);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("window-rule {"), "window-rule block expected");
        assert!(kdl.contains(r#"match app-id="^firefox$""#));
        assert!(kdl.contains("open-floating true"));
        assert!(kdl.contains("opacity 0.85"));
        assert!(kdl.contains(r#"block-out-from "screencast""#));
    }

    #[test]
    fn window_rule_size_limits_written() {
        let mut cfg = SettingsConfig::default();
        let mut r = WindowRule::default();
        r.match_app_id = "obs".into();
        r.min_width = 876;
        cfg.window_rules.push(r);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("min-width 876"));
    }

    #[test]
    fn window_rule_empty_when_all_defaults() {
        let mut cfg = SettingsConfig::default();
        cfg.window_rules.push(WindowRule::default());
        let kdl = generate_kdl(&cfg);
        assert!(!kdl.contains("window-rule"), "empty rule should not produce KDL block");
    }

    #[test]
    fn layer_rule_written_with_shadow_and_radius() {
        let mut cfg = SettingsConfig::default();
        let mut r = LayerRule::default();
        r.match_namespace = "^launcher$".into();
        r.shadow = TriState::On;
        r.geometry_corner_radius = 12.0;
        cfg.layer_rules.push(r);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("layer-rule {"), "layer-rule block expected");
        assert!(kdl.contains(r#"match namespace="^launcher$""#));
        assert!(kdl.contains("shadow {\n        on\n    }"));
        assert!(kdl.contains("geometry-corner-radius 12.0"));
    }

    #[test]
    fn layer_rule_block_out_from_screen_capture() {
        let mut cfg = SettingsConfig::default();
        let mut r = LayerRule::default();
        r.match_namespace = "^notifications$".into();
        r.block_out_from = BlockOutFrom::ScreenCapture;
        cfg.layer_rules.push(r);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains(r#"block-out-from "screen-capture""#));
    }

    #[test]
    fn tristate_roundtrip() {
        assert_eq!(TriState::from_index(0), TriState::Default);
        assert_eq!(TriState::from_index(1), TriState::On);
        assert_eq!(TriState::from_index(2), TriState::Off);
        assert_eq!(TriState::On.to_index(), 1);
        assert_eq!(TriState::Off.to_index(), 2);
    }

    #[test]
    fn block_out_from_roundtrip() {
        assert_eq!(BlockOutFrom::from_index(0), BlockOutFrom::None);
        assert_eq!(BlockOutFrom::from_index(1), BlockOutFrom::Screencast);
        assert_eq!(BlockOutFrom::from_index(2), BlockOutFrom::ScreenCapture);
        assert_eq!(BlockOutFrom::Screencast.as_kdl_str(), "screencast");
        assert_eq!(BlockOutFrom::ScreenCapture.as_kdl_str(), "screen-capture");
    }

    // ── Keybindings ───────────────────────────────────────────────────────────

    #[test]
    fn keybind_spawn_written_correctly() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+T".into();
        b.action = "spawn".into();
        b.action_args = "alacritty -e fish".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains(r#"Mod+T { spawn "alacritty" "-e" "fish"; }"#), "got:\n{kdl}");
    }

    #[test]
    fn keybind_spawn_sh_written_correctly() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+E".into();
        b.action = "spawn-sh".into();
        b.action_args = "notify-send hello world".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains(r#"Mod+E { spawn-sh "notify-send hello world"; }"#), "got:\n{kdl}");
    }

    #[test]
    fn keybind_no_arg_action() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+Left".into();
        b.action = "focus-column-left".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("Mod+Left { focus-column-left; }"), "got:\n{kdl}");
    }

    #[test]
    fn keybind_cooldown_and_no_repeat() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+WheelScrollDown".into();
        b.action = "focus-workspace-down".into();
        b.repeat = false;
        b.cooldown_ms = 150;
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("Mod+WheelScrollDown repeat=false cooldown-ms=150 { focus-workspace-down; }"),
            "got:\n{kdl}"
        );
    }

    #[test]
    fn keybind_allow_when_locked() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "XF86AudioMute".into();
        b.action = "spawn".into();
        b.action_args = "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle".into();
        b.allow_when_locked = true;
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("allow-when-locked=true"), "got:\n{kdl}");
    }

    #[test]
    fn keybind_empty_key_skipped() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "".into();
        b.action = "close-window".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(!kdl.contains("binds {"), "empty-key bind must be skipped: {kdl}");
    }

    #[test]
    fn keybind_spawn_empty_args_skipped() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+T".into();
        b.action = "spawn".into();
        b.action_args = "".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(!kdl.contains("binds {"), "spawn with no args must be skipped: {kdl}");
    }

    #[test]
    fn sanitise_keybind_key_strips_injection() {
        // Braces, semicolons, and spaces are stripped, preventing KDL injection.
        assert_eq!(
            sanitise_keybind_key(r#"Mod+T { evil; }"#),
            "Mod+Tevil"
        );
        assert_eq!(sanitise_keybind_key("XF86AudioMute"), "XF86AudioMute");
        assert_eq!(sanitise_keybind_key("ISO_Level3_Shift"), "ISO_Level3_Shift");
        assert_eq!(sanitise_keybind_key("Super+Alt+L"), "Super+Alt+L");
    }

    #[test]
    fn action_args_hint_spawn_nonempty() {
        assert!(!action_args_hint("spawn").is_empty());
        assert!(!action_args_hint("spawn-sh").is_empty());
        assert!(action_args_hint("close-window").is_empty());
    }

    // ── Bind line parser ──────────────────────────────────────────────────────

    #[test]
    fn parse_bind_line_simple_action() {
        let b = parse_bind_line("Mod+Left { focus-column-left; }").unwrap();
        assert_eq!(b.key, "Mod+Left");
        assert_eq!(b.action, "focus-column-left");
        assert!(b.action_args.is_empty());
        assert!(b.repeat);
        assert_eq!(b.cooldown_ms, 0);
        assert!(!b.allow_when_locked);
    }

    #[test]
    fn parse_bind_line_spawn_args() {
        let b = parse_bind_line(r#"Mod+T { spawn "alacritty"; }"#).unwrap();
        assert_eq!(b.key, "Mod+T");
        assert_eq!(b.action, "spawn");
        assert_eq!(b.action_args, "alacritty");
    }

    #[test]
    fn parse_bind_line_spawn_multi_args() {
        let b = parse_bind_line(
            r#"XF86MonBrightnessUp { spawn "brightnessctl" "--class=backlight" "set" "+10%"; }"#,
        )
        .unwrap();
        assert_eq!(b.action, "spawn");
        assert_eq!(b.action_args, "brightnessctl --class=backlight set +10%");
    }

    #[test]
    fn parse_bind_line_spawn_sh() {
        let b = parse_bind_line(
            r#"XF86AudioMute allow-when-locked=true { spawn-sh "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"; }"#,
        )
        .unwrap();
        assert_eq!(b.action, "spawn-sh");
        assert_eq!(b.action_args, "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle");
        assert!(b.allow_when_locked);
    }

    #[test]
    fn parse_bind_line_repeat_false() {
        let b = parse_bind_line("Mod+Q repeat=false { close-window; }").unwrap();
        assert_eq!(b.key, "Mod+Q");
        assert!(!b.repeat);
    }

    #[test]
    fn parse_bind_line_cooldown() {
        let b = parse_bind_line(
            "Mod+WheelScrollDown cooldown-ms=150 { focus-workspace-down; }",
        )
        .unwrap();
        assert_eq!(b.cooldown_ms, 150);
        assert_eq!(b.action, "focus-workspace-down");
    }

    #[test]
    fn parse_bind_line_action_with_index() {
        let b = parse_bind_line("Mod+1 { focus-workspace 1; }").unwrap();
        assert_eq!(b.action, "focus-workspace");
        assert_eq!(b.action_args, "1");
    }

    #[test]
    fn parse_bind_line_comment_returns_none() {
        assert!(parse_bind_line("// Mod+T { spawn \"alacritty\"; }").is_none());
        assert!(parse_bind_line("   // commented out").is_none());
        assert!(parse_bind_line("").is_none());
    }

    #[test]
    fn parse_bind_line_hotkey_overlay_title_ignored() {
        // hotkey-overlay-title is a property we intentionally skip.
        let b = parse_bind_line(
            r#"Mod+T hotkey-overlay-title="Open alacritty" { spawn "alacritty"; }"#,
        )
        .unwrap();
        assert_eq!(b.key, "Mod+T");
        assert_eq!(b.action, "spawn");
        assert_eq!(b.action_args, "alacritty");
    }
}

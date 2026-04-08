// GTK4 UI for the Quick Settings overlay — zero business logic.
//
// `QuickSettingsWindow` is a layer-shell window anchored top-right that shows:
//   • Header  — avatar initials, username@hostname, lock button
//   • Tile grid — 10 toggle tiles in 2 columns
//   • Sliders   — Brightness, Volume
//   • Footer    — Settings, Displays, Log out

use std::cell::RefCell;
use std::process::Child;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gtk4::{
    glib,
    prelude::*,
    Box as GtkBox, Button, Label, Orientation, Scale, Window,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::quick_settings_backend::{self as qs, PowerProfile, QsState};

// ── CSS ───────────────────────────────────────────────────────────────────────

const QS_CSS: &str = r#"
.qs-window {
    background: rgba(15, 15, 25, 0.92);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 14px;
    padding: 14px;
    font-family: "Inter", "Noto Sans", sans-serif;
}
/* ── Header ── */
.qs-avatar {
    background: #3d59a1;
    border-radius: 20px;
    min-width: 36px;
    min-height: 36px;
    font-size: 14px;
    font-weight: bold;
    color: #c0caf5;
}
.qs-user-name { font-size: 13px; font-weight: bold; color: #c0caf5; }
.qs-host-name { font-size: 11px; color: #565f89; }
.qs-lock-btn  { background: transparent; border: none; color: #565f89;
                font-size: 16px; padding: 4px 8px; border-radius: 8px; }
.qs-lock-btn:hover { background: rgba(255,255,255,0.08); color: #c0caf5; }

/* ── Tiles ── */
.qs-tile {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 10px 6px;
    min-width: 100px;
    min-height: 72px;
}
.qs-tile:hover { background: rgba(255,255,255,0.10); }
.qs-tile-active {
    background: rgba(61, 89, 161, 0.60);
    border-color: #7aa2f7;
}
.qs-tile-active:hover { background: rgba(61, 89, 161, 0.75); }
.qs-tile-icon { font-size: 20px; }
.qs-tile-name { font-size: 11px; font-weight: bold; color: #c0caf5; }
.qs-tile-desc { font-size: 9px; color: #565f89; }

/* ── Sliders ── */
.qs-slider-label { font-size: 11px; color: #a9b1d6; min-width: 80px; }
scale trough  { background: rgba(255,255,255,0.12); border-radius: 4px; min-height: 6px; }
scale highlight { background: #7aa2f7; border-radius: 4px; }

/* ── Footer ── */
.qs-foot-btn {
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.09);
    border-radius: 10px;
    color: #a9b1d6;
    font-size: 12px;
    padding: 6px 10px;
}
.qs-foot-btn:hover { background: rgba(255,255,255,0.12); color: #c0caf5; }
"#;

// ── Helper: tile widget ────────────────────────────────────────────────────────

struct TileRef {
    btn: Button,
    desc: Label,
}

fn build_tile(icon: &str, name: &str, desc: &str, active: bool) -> TileRef {
    let inner = GtkBox::new(Orientation::Vertical, 2);
    inner.set_halign(gtk4::Align::Center);
    inner.set_valign(gtk4::Align::Center);

    let icon_lbl = Label::new(Some(icon));
    icon_lbl.add_css_class("qs-tile-icon");

    let name_lbl = Label::new(Some(name));
    name_lbl.add_css_class("qs-tile-name");

    let desc_lbl = Label::new(Some(desc));
    desc_lbl.add_css_class("qs-tile-desc");
    desc_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);

    inner.append(&icon_lbl);
    inner.append(&name_lbl);
    inner.append(&desc_lbl);

    let btn = Button::new();
    btn.set_child(Some(&inner));
    btn.add_css_class("qs-tile");
    if active {
        btn.add_css_class("qs-tile-active");
    }

    TileRef { btn, desc: desc_lbl }
}

fn set_tile_active(tile: &TileRef, active: bool) {
    if active {
        tile.btn.add_css_class("qs-tile-active");
    } else {
        tile.btn.remove_css_class("qs-tile-active");
    }
}

// ── QuickSettingsWindow ───────────────────────────────────────────────────────

/// Full Quick Settings overlay window.
pub struct QuickSettingsWindow {
    window: Window,
    state: Rc<RefCell<QsState>>,
    night_light_child: Rc<RefCell<Option<Child>>>,
    idle_child: Rc<RefCell<Option<Child>>>,
    // Tile widget handles for CSS/desc updates
    tile_wifi: TileRef,
    tile_eth: TileRef,
    tile_vpn: TileRef,
    tile_bt: TileRef,
    tile_nl: TileRef,
    tile_dnd: TileRef,
    tile_kb: TileRef,
    tile_mic: TileRef,
    tile_pwr: TileRef,
    tile_idle: TileRef,
    // Slider handles
    brightness_scale: Scale,
    volume_scale: Scale,
}

impl QuickSettingsWindow {
    pub fn new(app: &gtk4::Application) -> Rc<Self> {
        // Load CSS
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(QS_CSS);
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("display"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let state = Rc::new(RefCell::new(QsState::default()));

        let window = Window::builder()
            .application(app)
            .decorated(false)
            .resizable(false)
            .build();
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_margin(Edge::Top, 42); // below panel
        window.set_margin(Edge::Right, 8);
        window.set_exclusive_zone(0);
        // Receive key events when the window is interacted with (required for Escape).
        window.set_keyboard_mode(KeyboardMode::OnDemand);

        // Escape closes the window
        let key_ctrl = gtk4::EventControllerKey::new();
        {
            let win_c = window.clone();
            key_ctrl.connect_key_pressed(move |_, key, _, _| {
                if key == gtk4::gdk::Key::Escape {
                    win_c.set_visible(false);
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
        }
        window.add_controller(key_ctrl);

        let s = state.borrow();

        // ── Build all tiles ───────────────────────────────────────────────
        let tile_wifi = build_tile(
            "📶",
            "Wi-Fi",
            if s.wifi_active { &s.wifi_ssid } else { "Off" },
            s.wifi_active,
        );
        let tile_eth = build_tile("🔌", "Ethernet", if s.ethernet_active { "Connected" } else { "Off" }, s.ethernet_active);
        let tile_vpn = build_tile("🛡", "VPN", if s.vpn_active { "On" } else { "Off" }, s.vpn_active);
        let tile_bt = build_tile("🔵", "Bluetooth", if s.bt_active { "On" } else { "Off" }, s.bt_active);
        let tile_nl = build_tile("🌙", "Night Light", if s.night_light { "On" } else { "Off" }, s.night_light);
        let tile_dnd = build_tile("🔕", "Do Not Disturb", if s.dnd { "On" } else { "Off" }, s.dnd);
        let tile_kb = build_tile("⌨", "Keyboard", if s.kb_layout.is_empty() { "—" } else { &s.kb_layout }, false);
        let tile_mic = build_tile(
            if s.mic_muted { "🎙️" } else { "🎙" },
            "Microphone",
            if s.mic_muted { "Muted" } else { "Active" },
            s.mic_muted,
        );
        let tile_pwr = build_tile("⚡", "Power", s.power_profile.label(), false);
        let tile_idle = build_tile("☕", "Idle Inhibit", if s.idle_inhibited { "Active" } else { "Off" }, s.idle_inhibited);
        drop(s);

        // ── Sliders ───────────────────────────────────────────────────────
        let has_brightness = qs::brightness_available();

        let brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        brightness_scale.set_hexpand(true);
        brightness_scale.set_draw_value(false);

        let volume_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        volume_scale.set_hexpand(true);
        volume_scale.set_draw_value(false);

        // ── Assemble layout ───────────────────────────────────────────────
        let root = GtkBox::new(Orientation::Vertical, 10);
        root.add_css_class("qs-window");
        root.set_size_request(300, -1);

        root.append(&build_header_row());
        root.append(&gtk4::Separator::new(Orientation::Horizontal));
        root.append(&build_tile_grid(
            &tile_wifi, &tile_eth, &tile_vpn, &tile_bt, &tile_nl,
            &tile_dnd, &tile_kb, &tile_mic, &tile_pwr, &tile_idle,
        ));
        root.append(&gtk4::Separator::new(Orientation::Horizontal));
        if has_brightness { root.append(&build_slider_row("☀ Brightness", &brightness_scale)); }
        root.append(&build_slider_row("🔊 Volume", &volume_scale));
        root.append(&gtk4::Separator::new(Orientation::Horizontal));
        root.append(&build_footer_row(app));

        window.set_child(Some(&root));

        let this = Rc::new(Self {
            window,
            state,
            night_light_child: Rc::new(RefCell::new(None)),
            idle_child: Rc::new(RefCell::new(None)),
            tile_wifi,
            tile_eth,
            tile_vpn,
            tile_bt,
            tile_nl,
            tile_dnd,
            tile_kb,
            tile_mic,
            tile_pwr,
            tile_idle,
            brightness_scale,
            volume_scale,
        });

        this.wire_tiles();
        this.wire_sliders();
        this
    }

    /// Show if hidden (after refreshing state); hide if visible.
    pub fn toggle(self: &Rc<Self>) {
        if self.window.is_visible() {
            self.window.set_visible(false);
        } else {
            self.window.present();
            // refresh() is non-blocking: heavy work is on a background thread.
            self.refresh();
        }
    }

    /// Returns the current Do Not Disturb flag.
    pub fn is_dnd(&self) -> bool {
        self.state.borrow().dnd
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Kick off a background load of `QsState`; results are applied on the GTK
    /// main thread via `glib::MainContext::channel` — no blocking on the GTK thread.
    fn refresh(self: &Rc<Self>) {
        let result: Arc<Mutex<Option<QsState>>> = Arc::new(Mutex::new(None));
        let writer = Arc::clone(&result);
        std::thread::spawn(move || {
            *writer.lock().unwrap() = Some(QsState::load());
        });
        let weak = Rc::downgrade(self);
        glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
            if let Some(fresh) = result.lock().unwrap().take() {
                if let Some(this) = weak.upgrade() {
                    this.apply_fresh_state(fresh);
                }
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }

    /// Apply a freshly-loaded `QsState` snapshot to all widgets. Runs on GTK thread.
    fn apply_fresh_state(&self, fresh: QsState) {
        let wifi_desc = if fresh.wifi_active { fresh.wifi_ssid.clone() } else { "Off".into() };
        set_tile_active(&self.tile_wifi, fresh.wifi_active);
        self.tile_wifi.desc.set_text(&wifi_desc);

        set_tile_active(&self.tile_eth, fresh.ethernet_active);
        self.tile_eth.desc.set_text(if fresh.ethernet_active { "Connected" } else { "Off" });

        set_tile_active(&self.tile_vpn, fresh.vpn_active);
        self.tile_vpn.desc.set_text(if fresh.vpn_active { "On" } else { "Off" });

        set_tile_active(&self.tile_bt, fresh.bt_active);
        self.tile_bt.desc.set_text(if fresh.bt_active { "On" } else { "Off" });

        // night_light, dnd, idle are in-process; preserve current values.
        let (nl, dnd, idle) = {
            let s = self.state.borrow();
            (s.night_light, s.dnd, s.idle_inhibited)
        };
        set_tile_active(&self.tile_nl, nl);
        self.tile_nl.desc.set_text(if nl { "On" } else { "Off" });

        set_tile_active(&self.tile_dnd, dnd);
        self.tile_dnd.desc.set_text(if dnd { "On" } else { "Off" });

        let kb_desc = if fresh.kb_layout.is_empty() { "—".into() } else { fresh.kb_layout.clone() };
        self.tile_kb.desc.set_text(&kb_desc);

        set_tile_active(&self.tile_mic, fresh.mic_muted);
        self.tile_mic.desc.set_text(if fresh.mic_muted { "Muted" } else { "Active" });

        self.tile_pwr.desc.set_text(fresh.power_profile.label());

        set_tile_active(&self.tile_idle, idle);
        self.tile_idle.desc.set_text(if idle { "Active" } else { "Off" });

        self.brightness_scale.set_value(fresh.brightness as f64);
        self.volume_scale.set_value(fresh.volume as f64);

        // Commit fresh state (preserve in-process fields).
        let mut s = self.state.borrow_mut();
        s.wifi_active = fresh.wifi_active;
        s.wifi_ssid = fresh.wifi_ssid;
        s.ethernet_active = fresh.ethernet_active;
        s.vpn_active = fresh.vpn_active;
        s.bt_active = fresh.bt_active;
        s.kb_layout = fresh.kb_layout;
        s.mic_muted = fresh.mic_muted;
        s.power_profile = fresh.power_profile;
        s.brightness = fresh.brightness;
        s.volume = fresh.volume;
    }

    fn wire_tiles(self: &Rc<Self>) {
        // WiFi — optimistic update; nmcli runs off-thread.
        {
            let state = Rc::clone(&self.state);
            let btn = self.tile_wifi.btn.clone();
            let desc = self.tile_wifi.desc.clone();
            self.tile_wifi.btn.connect_clicked(move |_| {
                let cur = state.borrow().wifi_active;
                let new = !cur;
                state.borrow_mut().wifi_active = new;
                if new { btn.add_css_class("qs-tile-active"); }
                else   { btn.remove_css_class("qs-tile-active"); }
                desc.set_text(if new { "Enabling…" } else { "Off" });

                // Arc flag: None=pending, Some(ok)
                let ok_flag: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
                let writer = Arc::clone(&ok_flag);
                std::thread::spawn(move || {
                    let ok = qs::toggle_wifi(cur)
                        .map_err(|e| log::warn!("wifi toggle: {e}"))
                        .is_ok();
                    *writer.lock().unwrap() = Some(ok);
                });
                let btn2 = btn.clone();
                let desc2 = desc.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
                    if let Some(ok) = ok_flag.lock().unwrap().take() {
                        if ok && new {
                            desc2.set_text("On"); // refresh() shows real SSID on next open
                        } else if !ok {
                            if cur { btn2.add_css_class("qs-tile-active"); }
                            else   { btn2.remove_css_class("qs-tile-active"); }
                            desc2.set_text(if cur { "On" } else { "Off" });
                        }
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
            });
        }

        // Ethernet — read-only display
        self.tile_eth.btn.set_sensitive(false);

        // VPN — read-only display
        self.tile_vpn.btn.set_sensitive(false);

        // Bluetooth — optimistic update; rfkill runs off-thread.
        {
            let state = Rc::clone(&self.state);
            let btn = self.tile_bt.btn.clone();
            let desc = self.tile_bt.desc.clone();
            self.tile_bt.btn.connect_clicked(move |_| {
                let cur = state.borrow().bt_active;
                let new = !cur;
                state.borrow_mut().bt_active = new;
                if new { btn.add_css_class("qs-tile-active"); }
                else   { btn.remove_css_class("qs-tile-active"); }
                desc.set_text(if new { "On" } else { "Off" });

                let ok_flag: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
                let writer = Arc::clone(&ok_flag);
                std::thread::spawn(move || {
                    let ok = qs::toggle_bt(cur)
                        .map_err(|e| log::warn!("bt toggle: {e}"))
                        .is_ok();
                    *writer.lock().unwrap() = Some(ok);
                });
                let btn2 = btn.clone();
                let desc2 = desc.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
                    if let Some(ok) = ok_flag.lock().unwrap().take() {
                        if !ok {
                            if cur { btn2.add_css_class("qs-tile-active"); }
                            else   { btn2.remove_css_class("qs-tile-active"); }
                            desc2.set_text(if cur { "On" } else { "Off" });
                        }
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
            });
        }

        // Night Light — spawns/kills gammastep; fast enough to stay on GTK thread.
        {
            let state = Rc::clone(&self.state);
            let child_ref = Rc::clone(&self.night_light_child);
            let btn = self.tile_nl.btn.clone();
            let desc = self.tile_nl.desc.clone();
            self.tile_nl.btn.connect_clicked(move |_| {
                let cur = state.borrow().night_light;
                match qs::toggle_night_light(cur, &mut child_ref.borrow_mut()) {
                    Ok(new) => {
                        state.borrow_mut().night_light = new;
                        if new { btn.add_css_class("qs-tile-active"); }
                        else   { btn.remove_css_class("qs-tile-active"); }
                        desc.set_text(if new { "On" } else { "Off" });
                    }
                    Err(e) => log::warn!("night light toggle: {e}"),
                }
            });
        }

        // DND — in-process only; instant.
        {
            let state = Rc::clone(&self.state);
            let btn = self.tile_dnd.btn.clone();
            let desc = self.tile_dnd.desc.clone();
            self.tile_dnd.btn.connect_clicked(move |_| {
                let new = !state.borrow().dnd;
                state.borrow_mut().dnd = new;
                if new { btn.add_css_class("qs-tile-active"); }
                else   { btn.remove_css_class("qs-tile-active"); }
                desc.set_text(if new { "On" } else { "Off" });
            });
        }

        // Keyboard layout — read-only
        self.tile_kb.btn.set_sensitive(false);

        // Mic mute — optimistic update; pactl runs off-thread.
        {
            let state = Rc::clone(&self.state);
            let btn = self.tile_mic.btn.clone();
            let desc = self.tile_mic.desc.clone();
            self.tile_mic.btn.connect_clicked(move |_| {
                let cur = state.borrow().mic_muted;
                let new = !cur;
                state.borrow_mut().mic_muted = new;
                if new { btn.add_css_class("qs-tile-active"); }
                else   { btn.remove_css_class("qs-tile-active"); }
                desc.set_text(if new { "Muted" } else { "Active" });

                // Store Option<Option<bool>>: None=pending, Some(Some(actual))=ok, Some(None)=err
                let result_flag: Arc<Mutex<Option<Option<bool>>>> = Arc::new(Mutex::new(None));
                let writer = Arc::clone(&result_flag);
                std::thread::spawn(move || {
                    let actual = qs::toggle_mic_mute()
                        .map_err(|e| log::warn!("mic mute toggle: {e}"))
                        .ok();
                    *writer.lock().unwrap() = Some(actual);
                });
                let btn2 = btn.clone();
                let desc2 = desc.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
                    if let Some(result) = result_flag.lock().unwrap().take() {
                        match result {
                            Some(actual) if actual != new => {
                                if actual { btn2.add_css_class("qs-tile-active"); }
                                else      { btn2.remove_css_class("qs-tile-active"); }
                                desc2.set_text(if actual { "Muted" } else { "Active" });
                            }
                            None => {
                                if cur { btn2.add_css_class("qs-tile-active"); }
                                else   { btn2.remove_css_class("qs-tile-active"); }
                                desc2.set_text(if cur { "Muted" } else { "Active" });
                            }
                            _ => {}
                        }
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
            });
        }

        // Power profile — optimistic cycle; powerprofilesctl runs off-thread.
        {
            let state = Rc::clone(&self.state);
            let btn = self.tile_pwr.btn.clone();
            let desc = self.tile_pwr.desc.clone();
            self.tile_pwr.btn.connect_clicked(move |_| {
                let cur = state.borrow().power_profile;
                let next = cur.next();
                state.borrow_mut().power_profile = next;
                desc.set_text(next.label());
                if next != PowerProfile::Balanced { btn.add_css_class("qs-tile-active"); }
                else { btn.remove_css_class("qs-tile-active"); }

                let ok_flag: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
                let writer = Arc::clone(&ok_flag);
                std::thread::spawn(move || {
                    let ok = qs::cycle_power_profile(cur)
                        .map_err(|e| log::warn!("power profile cycle: {e}"))
                        .is_ok();
                    *writer.lock().unwrap() = Some(ok);
                });
                let btn2 = btn.clone();
                let desc2 = desc.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
                    if let Some(ok) = ok_flag.lock().unwrap().take() {
                        if !ok {
                            desc2.set_text(cur.label());
                            if cur != PowerProfile::Balanced { btn2.add_css_class("qs-tile-active"); }
                            else { btn2.remove_css_class("qs-tile-active"); }
                        }
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
            });
        }

        // Idle inhibitor — spawns/kills systemd-inhibit; fast enough to stay on GTK thread.
        {
            let state = Rc::clone(&self.state);
            let child_ref = Rc::clone(&self.idle_child);
            let btn = self.tile_idle.btn.clone();
            let desc = self.tile_idle.desc.clone();
            self.tile_idle.btn.connect_clicked(move |_| {
                let cur = state.borrow().idle_inhibited;
                match qs::toggle_idle_inhibitor(cur, &mut child_ref.borrow_mut()) {
                    Ok(new) => {
                        state.borrow_mut().idle_inhibited = new;
                        if new { btn.add_css_class("qs-tile-active"); }
                        else   { btn.remove_css_class("qs-tile-active"); }
                        desc.set_text(if new { "Active" } else { "Off" });
                    }
                    Err(e) => log::warn!("idle inhibitor toggle: {e}"),
                }
            });
        }
    }

    fn wire_sliders(self: &Rc<Self>) {
        // Brightness — update state immediately; write to hardware off-thread.
        {
            let state = Rc::clone(&self.state);
            self.brightness_scale.connect_value_changed(move |scale| {
                let pct = scale.value().round() as u8;
                state.borrow_mut().brightness = pct;
                std::thread::spawn(move || {
                    if let Err(e) = qs::set_brightness(pct) {
                        log::warn!("brightness: {e}");
                    }
                });
            });
        }

        // Volume — update state immediately; pactl runs off-thread.
        {
            let state = Rc::clone(&self.state);
            self.volume_scale.connect_value_changed(move |scale| {
                let pct = scale.value().round() as u8;
                state.borrow_mut().volume = pct;
                std::thread::spawn(move || {
                    if let Err(e) = qs::set_volume_abs(pct) {
                        log::warn!("volume: {e}");
                    }
                });
            });
        }
    }
}

// ── Layout helper functions ───────────────────────────────────────────────────

fn build_header_row() -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_valign(gtk4::Align::Center);

    // Avatar — first letter of username
    let username = std::env::var("USER").unwrap_or_else(|_| "user".into());
    let hostname = std::fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "niri".into())
        .trim()
        .to_owned();
    let initial = username.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "U".into());

    let avatar = Label::new(Some(&initial));
    avatar.add_css_class("qs-avatar");
    avatar.set_justify(gtk4::Justification::Center);
    avatar.set_halign(gtk4::Align::Center);
    avatar.set_valign(gtk4::Align::Center);

    let user_box = GtkBox::new(Orientation::Vertical, 0);
    user_box.set_hexpand(true);
    let name_lbl = Label::new(Some(&username));
    name_lbl.add_css_class("qs-user-name");
    name_lbl.set_halign(gtk4::Align::Start);
    let host_lbl = Label::new(Some(&hostname));
    host_lbl.add_css_class("qs-host-name");
    host_lbl.set_halign(gtk4::Align::Start);
    user_box.append(&name_lbl);
    user_box.append(&host_lbl);

    let lock_btn = Button::with_label("🔒");
    lock_btn.add_css_class("qs-lock-btn");
    lock_btn.connect_clicked(|_| {
        qs::launch("hyprlock || swaylock");
    });

    row.append(&avatar);
    row.append(&user_box);
    row.append(&lock_btn);
    row
}

#[allow(clippy::too_many_arguments)]
fn build_tile_grid(
    wifi: &TileRef,
    eth: &TileRef,
    vpn: &TileRef,
    bt: &TileRef,
    nl: &TileRef,
    dnd: &TileRef,
    kb: &TileRef,
    mic: &TileRef,
    pwr: &TileRef,
    idle: &TileRef,
) -> GtkBox {
    let grid = gtk4::Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(8);
    grid.set_column_homogeneous(true);

    let tiles: [&TileRef; 10] = [wifi, eth, vpn, bt, nl, dnd, kb, mic, pwr, idle];
    for (i, tile) in tiles.iter().enumerate() {
        let col = (i % 2) as i32;
        let row = (i / 2) as i32;
        grid.attach(&tile.btn, col, row, 1, 1);
    }

    let wrapper = GtkBox::new(Orientation::Horizontal, 0);
    wrapper.append(&grid);
    wrapper
}

fn build_slider_row(label: &str, scale: &Scale) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_valign(gtk4::Align::Center);
    let lbl = Label::new(Some(label));
    lbl.add_css_class("qs-slider-label");
    lbl.set_halign(gtk4::Align::Start);
    row.append(&lbl);
    row.append(scale);
    row
}

fn build_footer_row(app: &gtk4::Application) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 6);

    let settings_btn = Button::with_label("⚙ Settings");
    settings_btn.add_css_class("qs-foot-btn");
    settings_btn.set_hexpand(true);
    settings_btn.connect_clicked(|_| {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).arg("--settings").spawn();
        } else {
            log::warn!("could not determine current exe path to launch settings");
        }
    });

    let displays_btn = Button::with_label("🖥 Displays");
    displays_btn.add_css_class("qs-foot-btn");
    displays_btn.set_hexpand(true);
    displays_btn.connect_clicked(|_| {
        qs::launch("wdisplays");
    });

    let logout_btn = Button::with_label("⏻ Log out");
    logout_btn.add_css_class("qs-foot-btn");
    logout_btn.set_hexpand(true);
    logout_btn.connect_clicked(|_| {
        qs::launch("niri msg action quit");
    });

    let power_btn = Button::with_label("🔴 Power");
    power_btn.add_css_class("qs-foot-btn");
    power_btn.set_hexpand(true);
    let app_c = app.clone();
    power_btn.connect_clicked(move |_| {
        crate::power_ui::show_power_dialog(&app_c);
    });

    row.append(&settings_btn);
    row.append(&displays_btn);
    row.append(&logout_btn);
    row.append(&power_btn);

    // — caller appends a sep before calling this,
    // but we also add a bit of top padding for visual breathing room.
    let wrapper = GtkBox::new(Orientation::Vertical, 0);
    wrapper.set_margin_top(2);
    wrapper.append(&row);
    wrapper
}

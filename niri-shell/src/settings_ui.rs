// settings_ui.rs — GTK4 niri Settings window.
//
// Sidebar navigation + stack of pages. Fully functional pages:
//   • behaviour  — input & compositor toggles
//   • layout     — gaps, decoration, focus-ring
// Stub pages for future work: keybindings, window-rules, outputs, software.
//
// Launched via `niri-shell --settings`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gio::ApplicationFlags;
use gtk4::prelude::*;
use gtk4::{
    glib, Align, Box as GtkBox, Button, CssProvider, DropDown, Entry, Label,
    Orientation, Scale, ScrolledWindow, Separator, Stack, Switch, Window,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};

use crate::settings_backend::{
    AccelProfile, AnimSettings, BlockOutFrom, CenterFocusedColumn, ClickMethod, DragSetting,
    Keybind, LayerRule, MouseScrollMethod, OutputConfig, OutputTransform, SettingsConfig,
    SwitchEventsSettings, TapButtonMap, TouchpadScrollMethod, TrackLayout, TriState, VrrMode,
    WindowRule, NIRI_ACTIONS, action_args_hint, action_needs_args, import_binds_from_niri_config,
};

// ── Row-tuple type aliases (avoids clippy::type_complexity) ───────────────────

type AnimBoolRow = (
    &'static str,
    &'static str,
    fn(&AnimSettings) -> bool,
    fn(&mut AnimSettings, bool),
);
type CfgU32Row = (
    &'static str,
    &'static str,
    u32,
    fn(&SettingsConfig) -> u32,
    fn(&mut SettingsConfig, u32),
);
type CfgBoolRow = (
    &'static str,
    &'static str,
    fn(&SettingsConfig) -> bool,
    fn(&mut SettingsConfig, bool),
);

// ── CSS ───────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
.settings-window {
    background-color: rgba(13, 14, 23, 0.98);
    color: #c0caf5;
}
.settings-sidebar {
    background-color: rgba(8, 9, 16, 1.0);
    border-right: 1px solid rgba(255, 255, 255, 0.07);
}
.sidebar-header {
    padding: 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
}
.sidebar-title {
    font-size: 13px;
    font-weight: 500;
    color: #c0caf5;
}
.sidebar-section-label {
    font-size: 10px;
    color: #3b3f5c;
    letter-spacing: 0.06em;
    padding: 8px 14px 4px;
}
.sidebar-item {
    border-radius: 7px;
    padding: 7px 10px;
    margin: 1px 8px;
    font-size: 12px;
    color: #565f89;
    background: transparent;
    border: none;
    box-shadow: none;
    min-height: 0;
}
.sidebar-item:hover {
    background-color: rgba(255, 255, 255, 0.04);
    color: #a0aad5;
}
.sidebar-item.active {
    background-color: rgba(122, 162, 247, 0.12);
    color: #c0caf5;
}
.settings-content {
    background-color: transparent;
}
.page-title {
    font-size: 15px;
    font-weight: 500;
    color: #c0caf5;
}
.page-sub {
    font-size: 11px;
    color: #565f89;
}
.settings-card {
    background-color: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 10px;
    margin-top: 10px;
}
.card-title {
    font-size: 10px;
    font-weight: 600;
    color: #7aa2f7;
    letter-spacing: 0.07em;
    padding: 10px 14px;
}
.settings-row {
    padding: 9px 14px;
}
.row-sep {
    background-color: rgba(255, 255, 255, 0.05);
    min-height: 1px;
}
.row-label {
    font-size: 12px;
    color: #c0caf5;
}
.row-sublabel {
    font-size: 10px;
    color: #565f89;
}
.save-btn {
    background-color: rgba(122, 162, 247, 0.18);
    border: 1px solid rgba(122, 162, 247, 0.35);
    border-radius: 8px;
    color: #7aa2f7;
    font-size: 12px;
    padding: 6px 18px;
    min-height: 0;
    margin-top: 10px;
    margin-bottom: 10px;
}
.save-btn:hover {
    background-color: rgba(122, 162, 247, 0.3);
}
.stub-label {
    font-size: 12px;
    color: #3b3f5c;
    padding: 20px 14px;
}
.color-entry {
    font-family: monospace;
    font-size: 12px;
    min-width: 100px;
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 6px;
    color: #c0caf5;
    padding: 4px 8px;
}
.ver-label {
    font-size: 10px;
    color: #3b3f5c;
    padding: 10px 14px;
}
"#;

// ── Public entry point ────────────────────────────────────────────────────────

/// Creates a standalone GTK Application and opens the settings window.
/// Called from `main()` when `--settings` is passed.
pub fn run() -> Result<(), crate::error::ShellError> {
    // Prefer the GL renderer — the Vulkan backend emits VK_ERROR_OUT_OF_DATE_KHR
    // on every resize and has no benefit for a simple settings dialog.
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "gl");
    }
    gtk4::init().map_err(|_| crate::error::ShellError::GtkInit)?;

    let app = gtk4::Application::builder()
        .application_id("org.niri.settings")
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(|app| {
        let win = SettingsWindow::new();
        win.window.set_application(Some(app));
        win.show();
    });

    // Pass only argv[0] so GTK doesn't choke on the `--settings` flag.
    let argv0 = std::env::args().next().unwrap_or_default();
    app.run_with_args(&[argv0]);
    Ok(())
}

// ── SettingsWindow ────────────────────────────────────────────────────────────

pub struct SettingsWindow {
    pub window: Window,
}

impl SettingsWindow {
    pub fn new() -> Self {
        let cfg = Rc::new(RefCell::new(
            crate::settings_backend::load().unwrap_or_default(),
        ));

        let window = Window::new();
        window.set_title(Some("niri settings"));
        window.set_default_size(840, 580);
        window.set_resizable(true);
        window.add_css_class("settings-window");

        // Apply CSS globally for this display.
        let provider = CssProvider::new();
        provider.load_from_string(CSS);
        gtk4::style_context_add_provider_for_display(
            &gtk4::prelude::WidgetExt::display(&window),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Root layout: sidebar | content
        let root = GtkBox::new(Orientation::Horizontal, 0);
        root.add_css_class("settings-window");

        // ── Build sidebar ──────────────────────────────────────────────────────
        let sidebar = self::build_sidebar();

        // ── Build page stack ───────────────────────────────────────────────────
        let stack = Rc::new(Stack::new());
        stack.add_named(&build_behaviour_page(Rc::clone(&cfg)), Some("behaviour"));
        stack.add_named(&build_input_page(Rc::clone(&cfg)), Some("input"));
        stack.add_named(&build_layout_page(Rc::clone(&cfg)), Some("layout"));
        stack.add_named(&build_animations_page(Rc::clone(&cfg)), Some("animations"));
        stack.add_named(&build_workspaces_page(Rc::clone(&cfg)), Some("workspaces"));
        stack.add_named(&build_gestures_page(Rc::clone(&cfg)), Some("gestures"));
        stack.add_named(&build_recent_windows_page(Rc::clone(&cfg)), Some("recent-windows"));
        stack.add_named(&build_miscellaneous_page(Rc::clone(&cfg)), Some("miscellaneous"));
        stack.add_named(&build_debug_page(Rc::clone(&cfg)), Some("debug"));
        stack.add_named(&build_keybindings_page(Rc::clone(&cfg)), Some("keybindings"));
        stack.add_named(&build_window_rules_page(Rc::clone(&cfg)), Some("window-rules"));
        stack.add_named(&build_layer_rules_page(Rc::clone(&cfg)), Some("layer-rules"));
        stack.add_named(&build_switch_events_page(Rc::clone(&cfg)), Some("switch-events"));
        stack.add_named(&build_outputs_page(Rc::clone(&cfg)), Some("outputs"));
        stack.add_named(
            &build_stub_page("software", "Software tools panel — coming soon"),
            Some("software"),
        );
        stack.set_visible_child_name("behaviour");

        // Wire sidebar buttons to the stack
        wire_sidebar(&sidebar.container, &stack);

        // ── Assemble root ──────────────────────────────────────────────────────
        root.append(&sidebar.container);

        let scroll = ScrolledWindow::new();
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&*stack));
        root.append(&scroll);

        window.set_child(Some(&root));

        Self { window }
    }

    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.present();
    }
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

struct Sidebar {
    container: GtkBox,
}

const NAV_ITEMS: &[(&str, &str)] = &[
    ("behaviour", "behaviour"),
    ("input", "input"),
    ("layout", "layout"),
    ("animations", "animations"),
    ("workspaces", "workspaces"),
    ("gestures", "gestures"),
    ("recent-windows", "recent windows"),
    ("miscellaneous", "miscellaneous"),
    ("debug", "debug"),
    ("keybindings", "keybindings"),
    ("window-rules", "window rules"),
    ("layer-rules", "layer rules"),
    ("switch-events", "switch events"),
    ("outputs", "outputs"),
    ("software", "software"),
];

fn build_sidebar() -> Sidebar {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.add_css_class("settings-sidebar");
    container.set_hexpand(false);
    container.set_vexpand(true);
    container.set_width_request(170);

    // Header
    let header = GtkBox::new(Orientation::Vertical, 2);
    header.add_css_class("sidebar-header");
    let title = Label::new(Some("settings"));
    title.add_css_class("sidebar-title");
    title.set_xalign(0.0);
    let sub = Label::new(Some("niri-shell"));
    sub.add_css_class("row-sublabel");
    sub.set_xalign(0.0);
    header.append(&title);
    header.append(&sub);
    container.append(&header);

    // Section label: "niri"
    let sec = Label::new(Some("niri"));
    sec.add_css_class("sidebar-section-label");
    sec.set_xalign(0.0);
    container.append(&sec);

    // Nav items (behaviour → software)
    for (page_name, label) in NAV_ITEMS {
        let btn = Button::with_label(label);
        btn.add_css_class("sidebar-item");
        btn.set_hexpand(true);
        // page is stored as widget name for later wiring
        btn.set_widget_name(page_name);
        container.append(&btn);
    }

    // Spacer + version
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    container.append(&spacer);

    let ver = Label::new(Some("niri-shell v0.1.0"));
    ver.add_css_class("ver-label");
    ver.set_xalign(0.0);
    container.append(&ver);

    Sidebar { container }
}

/// Connects sidebar button clicks to the stack and manages the active CSS class.
fn wire_sidebar(sidebar: &GtkBox, stack: &Rc<Stack>) {
    // Collect all sidebar-item buttons
    let mut buttons: Vec<Button> = Vec::new();
    let mut child = sidebar.first_child();
    while let Some(w) = child {
        if w.has_css_class("sidebar-item") {
            if let Ok(btn) = w.clone().downcast::<Button>() {
                buttons.push(btn);
            }
        }
        child = w.next_sibling();
    }

    let buttons = Rc::new(buttons);

    for btn in buttons.iter() {
        let page_name = btn.widget_name().to_string();
        let stack_c = Rc::clone(stack);
        let buttons_c = Rc::clone(&buttons);
        let btn_weak = btn.downgrade();
        btn.connect_clicked(move |_| {
            stack_c.set_visible_child_name(&page_name);
            for b in buttons_c.iter() {
                b.remove_css_class("active");
            }
            if let Some(b) = btn_weak.upgrade() {
                b.add_css_class("active");
            }
        });
    }

    // Activate the first button by default
    if let Some(first) = buttons.first() {
        first.add_css_class("active");
    }
}

// ── Behaviour page ────────────────────────────────────────────────────────────

fn build_behaviour_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();

    page.append(&page_header("behaviour", "compositor & input settings"));

    // Card: compositor
    let comp_card = card("compositor");
    append_toggle(
        &comp_card,
        "prefer no decorations",
        "ask apps to omit client-side decorations (prefer-no-csd)",
        cfg.borrow().misc.prefer_no_csd,
        with_cfg(Rc::clone(&cfg), |c, v| c.misc.prefer_no_csd = v),
    );
    append_toggle(
        &comp_card,
        "focus follows mouse",
        "focus windows as the pointer moves over them",
        cfg.borrow().input.focus_follows_mouse,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.focus_follows_mouse = v),
    );
    append_toggle(
        &comp_card,
        "workspace auto back-and-forth",
        "re-selecting the current workspace switches back to the previous one",
        cfg.borrow().input.workspace_auto_back_and_forth,
        with_cfg(Rc::clone(&cfg), |c, v| {
            c.input.workspace_auto_back_and_forth = v;
        }),
    );
    append_toggle(
        &comp_card,
        "warp mouse to focus",
        "move the cursor to newly focused windows",
        cfg.borrow().input.warp_mouse_to_focus,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.warp_mouse_to_focus = v),
    );
    append_toggle(
        &comp_card,
        "skip hotkey overlay at startup",
        "hide the 'Important Hotkeys' overlay when niri starts",
        cfg.borrow().misc.skip_hotkey_overlay,
        with_cfg(Rc::clone(&cfg), |c, v| c.misc.skip_hotkey_overlay = v),
    );
    append_toggle(
        &comp_card,
        "disable power key handling",
        "let systemd/logind handle the power button instead of niri",
        cfg.borrow().input.disable_power_key_handling,
        with_cfg(Rc::clone(&cfg), |c, v| {
            c.input.disable_power_key_handling = v;
        }),
    );
    page.append(&comp_card);

    page.append(&save_button(cfg));
    page
}

// ── Input page ────────────────────────────────────────────────────────────────

fn build_input_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("input", "keyboard, touchpad & mouse"));

    // ── Card: keyboard ────────────────────────────────────────────────────────
    let kb_card = card("keyboard");

    // XKB text entries
    type XkbRow = (&'static str, &'static str, fn(&SettingsConfig) -> &str, fn(&mut SettingsConfig, String));
    let xkb_rows: &[XkbRow] = &[
        ("layout", "e.g. us,de", |c| &c.input.keyboard_xkb_layout, |c, v| c.input.keyboard_xkb_layout = v),
        ("variant", "e.g. dvorak", |c| &c.input.keyboard_xkb_variant, |c, v| c.input.keyboard_xkb_variant = v),
        ("model", "e.g. pc105", |c| &c.input.keyboard_xkb_model, |c, v| c.input.keyboard_xkb_model = v),
        ("rules", "leave blank for default", |c| &c.input.keyboard_xkb_rules, |c, v| c.input.keyboard_xkb_rules = v),
        ("options", "e.g. compose:ralt,caps:escape", |c| &c.input.keyboard_xkb_options, |c, v| c.input.keyboard_xkb_options = v),
    ];
    for (label, hint, getter, setter) in xkb_rows {
        let sep = Separator::new(Orientation::Horizontal);
        sep.add_css_class("row-sep");
        kb_card.append(&sep);

        let row = row_box();
        let lbl_col = label_col(label, hint);
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_text(getter(&cfg.borrow()));
        entry.set_width_chars(20);
        entry.set_max_width_chars(40);
        let cfg_e = Rc::clone(&cfg);
        let setter_copy = *setter;
        entry.connect_changed(move |e| {
            setter_copy(&mut cfg_e.borrow_mut(), e.text().to_string());
        });
        row.append(&entry);
        kb_card.append(&row);
    }

    // Repeat delay slider
    {
        kb_card.append(&Separator::new(Orientation::Horizontal));
        let row = row_box();
        let lbl_col = label_col("repeat delay", "key-held delay before repeat starts (ms, 0=default 600)");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init_d = cfg.borrow().input.keyboard_repeat_delay;
        let val_lbl = Label::new(Some(&if init_d == 0 {
            "default".to_string()
        } else {
            format!("{init_d}ms")
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 2000.0, 50.0);
        scale.set_value(init_d as f64);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            cfg_s.borrow_mut().input.keyboard_repeat_delay = v;
            val_lbl_c.set_text(&if v == 0 {
                "default".to_string()
            } else {
                format!("{v}ms")
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        kb_card.append(&row);
    }

    // Repeat rate slider
    {
        kb_card.append(&Separator::new(Orientation::Horizontal));
        let row = row_box();
        let lbl_col = label_col("repeat rate", "key repeat speed in characters per second (0=default 25)");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init_r = cfg.borrow().input.keyboard_repeat_rate;
        let val_lbl = Label::new(Some(&if init_r == 0 {
            "default".to_string()
        } else {
            format!("{init_r} cps")
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(9);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        scale.set_value(init_r as f64);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            cfg_s.borrow_mut().input.keyboard_repeat_rate = v;
            val_lbl_c.set_text(&if v == 0 {
                "default".to_string()
            } else {
                format!("{v} cps")
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        kb_card.append(&row);
    }

    // Track layout dropdown
    {
        kb_card.append(&Separator::new(Orientation::Horizontal));
        let row = row_box();
        let lbl_col = label_col("track layout", "which layout to use when multiple are configured");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let options = gtk4::StringList::new(&["global", "window"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.keyboard_track_layout {
            TrackLayout::Global => 0,
            TrackLayout::Window => 1,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.keyboard_track_layout = match d.selected() {
                1 => TrackLayout::Window,
                _ => TrackLayout::Global,
            };
        });
        row.append(&dd);
        kb_card.append(&row);
    }

    // Numlock toggle
    kb_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &kb_card,
        "enable Num Lock at startup",
        "turn on Num Lock automatically when niri starts",
        cfg.borrow().input.numlock,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.numlock = v),
    );

    page.append(&kb_card);

    // ── Card: touchpad ────────────────────────────────────────────────────────
    let tp_card = card("touchpad");

    append_toggle(
        &tp_card,
        "disable touchpad",
        "turn off the touchpad entirely",
        cfg.borrow().input.touchpad_off,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_off = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "tap to click",
        "register a click when tapping the touchpad surface",
        cfg.borrow().input.touchpad_tap,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_tap = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "disable while typing",
        "pause touchpad input while keys are pressed (dwt)",
        cfg.borrow().input.touchpad_dwt,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_dwt = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "disable while trackpointing",
        "pause touchpad while an external pointing device is in use (dwtp)",
        cfg.borrow().input.touchpad_dwtp,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_dwtp = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Drag dropdown
    {
        let row = row_box();
        let lbl_col = label_col("drag", "enable or disable tap-to-drag");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let options = gtk4::StringList::new(&["default", "enabled", "disabled"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_drag {
            DragSetting::Default => 0,
            DragSetting::Enabled => 1,
            DragSetting::Disabled => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_drag = match d.selected() {
                1 => DragSetting::Enabled,
                2 => DragSetting::Disabled,
                _ => DragSetting::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "drag lock",
        "keep drag active after lifting the finger until tapped again",
        cfg.borrow().input.touchpad_drag_lock,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_drag_lock = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "natural scroll",
        "invert scroll direction so content follows the finger",
        cfg.borrow().input.touchpad_natural_scroll,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_natural_scroll = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Accel speed slider
    {
        let row = row_box();
        let lbl_col = label_col("accel speed", "pointer acceleration speed (-1.0 to 1.0, 0=default)");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init = cfg.borrow().input.touchpad_accel_speed;
        let val_lbl = Label::new(Some(&format!("{init:.1}")));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(5);

        let scale = Scale::with_range(Orientation::Horizontal, -1.0, 1.0, 0.1);
        scale.set_value(init);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 10.0).round() / 10.0;
            cfg_s.borrow_mut().input.touchpad_accel_speed = v;
            val_lbl_c.set_text(&format!("{v:.1}"));
        });

        row.append(&scale);
        row.append(&val_lbl);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Accel profile dropdown
    {
        let row = row_box();
        let lbl_col = label_col("accel profile", "pointer acceleration algorithm");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let options = gtk4::StringList::new(&["default", "adaptive", "flat"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_accel_profile {
            AccelProfile::Default => 0,
            AccelProfile::Adaptive => 1,
            AccelProfile::Flat => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_accel_profile = match d.selected() {
                1 => AccelProfile::Adaptive,
                2 => AccelProfile::Flat,
                _ => AccelProfile::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Scroll method dropdown
    {
        let row = row_box();
        let lbl_col = label_col("scroll method", "how scroll events are generated");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let options = gtk4::StringList::new(&["default", "two-finger", "edge", "on-button-down", "no-scroll"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_scroll_method {
            TouchpadScrollMethod::Default => 0,
            TouchpadScrollMethod::TwoFinger => 1,
            TouchpadScrollMethod::Edge => 2,
            TouchpadScrollMethod::OnButtonDown => 3,
            TouchpadScrollMethod::NoScroll => 4,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_scroll_method = match d.selected() {
                1 => TouchpadScrollMethod::TwoFinger,
                2 => TouchpadScrollMethod::Edge,
                3 => TouchpadScrollMethod::OnButtonDown,
                4 => TouchpadScrollMethod::NoScroll,
                _ => TouchpadScrollMethod::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Tap button map dropdown
    {
        let row = row_box();
        let lbl_col = label_col("tap button map", "mapping for 1/2/3 finger taps");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let options = gtk4::StringList::new(&["default", "left-right-middle", "left-middle-right"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_tap_button_map {
            TapButtonMap::Default => 0,
            TapButtonMap::LeftRightMiddle => 1,
            TapButtonMap::LeftMiddleRight => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_tap_button_map = match d.selected() {
                1 => TapButtonMap::LeftRightMiddle,
                2 => TapButtonMap::LeftMiddleRight,
                _ => TapButtonMap::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Click method dropdown
    {
        let row = row_box();
        let lbl_col = label_col("click method", "how physical button clicks are emitted");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let options = gtk4::StringList::new(&["default", "button-areas", "clickfinger"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_click_method {
            ClickMethod::Default => 0,
            ClickMethod::ButtonAreas => 1,
            ClickMethod::Clickfinger => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_click_method = match d.selected() {
                1 => ClickMethod::ButtonAreas,
                2 => ClickMethod::Clickfinger,
                _ => ClickMethod::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "disabled on external mouse",
        "turn off touchpad when a mouse is plugged in",
        cfg.borrow().input.touchpad_disabled_on_external_mouse,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_disabled_on_external_mouse = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "left-handed mode",
        "swap left and right buttons",
        cfg.borrow().input.touchpad_left_handed,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_left_handed = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "middle button emulation",
        "press left+right simultaneously to emit a middle click",
        cfg.borrow().input.touchpad_middle_emulation,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.touchpad_middle_emulation = v),
    );

    page.append(&tp_card);

    // ── Card: mouse ───────────────────────────────────────────────────────────
    let ms_card = card("mouse");

    append_toggle(
        &ms_card,
        "disable mouse",
        "turn off all mice",
        cfg.borrow().input.mouse_off,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.mouse_off = v),
    );
    ms_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &ms_card,
        "natural scroll",
        "invert the mouse wheel scrolling direction",
        cfg.borrow().input.mouse_natural_scroll,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.mouse_natural_scroll = v),
    );
    ms_card.append(&Separator::new(Orientation::Horizontal));

    // Mouse accel speed
    {
        let row = row_box();
        let lbl_col = label_col("accel speed", "pointer acceleration speed (-1.0 to 1.0, 0=default)");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init = cfg.borrow().input.mouse_accel_speed;
        let val_lbl = Label::new(Some(&format!("{init:.1}")));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(5);

        let scale = Scale::with_range(Orientation::Horizontal, -1.0, 1.0, 0.1);
        scale.set_value(init);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 10.0).round() / 10.0;
            cfg_s.borrow_mut().input.mouse_accel_speed = v;
            val_lbl_c.set_text(&format!("{v:.1}"));
        });

        row.append(&scale);
        row.append(&val_lbl);
        ms_card.append(&row);
    }

    ms_card.append(&Separator::new(Orientation::Horizontal));

    // Mouse accel profile
    {
        let row = row_box();
        let lbl_col = label_col("accel profile", "pointer acceleration algorithm");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let options = gtk4::StringList::new(&["default", "adaptive", "flat"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.mouse_accel_profile {
            AccelProfile::Default => 0,
            AccelProfile::Adaptive => 1,
            AccelProfile::Flat => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.mouse_accel_profile = match d.selected() {
                1 => AccelProfile::Adaptive,
                2 => AccelProfile::Flat,
                _ => AccelProfile::Default,
            };
        });
        row.append(&dd);
        ms_card.append(&row);
    }

    ms_card.append(&Separator::new(Orientation::Horizontal));

    // Mouse scroll method
    {
        let row = row_box();
        let lbl_col = label_col("scroll method", "how scroll events are generated");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let options = gtk4::StringList::new(&["default", "no-scroll", "two-finger", "edge", "on-button-down"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.mouse_scroll_method {
            MouseScrollMethod::Default => 0,
            MouseScrollMethod::NoScroll => 1,
            MouseScrollMethod::TwoFinger => 2,
            MouseScrollMethod::Edge => 3,
            MouseScrollMethod::OnButtonDown => 4,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.mouse_scroll_method = match d.selected() {
                1 => MouseScrollMethod::NoScroll,
                2 => MouseScrollMethod::TwoFinger,
                3 => MouseScrollMethod::Edge,
                4 => MouseScrollMethod::OnButtonDown,
                _ => MouseScrollMethod::Default,
            };
        });
        row.append(&dd);
        ms_card.append(&row);
    }

    ms_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &ms_card,
        "left-handed mode",
        "swap left and right buttons",
        cfg.borrow().input.mouse_left_handed,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.mouse_left_handed = v),
    );
    ms_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &ms_card,
        "middle button emulation",
        "press left+right simultaneously to emit a middle click",
        cfg.borrow().input.mouse_middle_emulation,
        with_cfg(Rc::clone(&cfg), |c, v| c.input.mouse_middle_emulation = v),
    );

    page.append(&ms_card);

    // ── Card: mod keys ────────────────────────────────────────────────────────
    let mk_card = card("mod keys");

    for (label, hint, getter, setter) in [
        (
            "mod-key",
            "modifier key used for niri actions (e.g. Super)",
            (|c: &SettingsConfig| c.input.mod_key.as_str()) as fn(&SettingsConfig) -> &str,
            (|c: &mut SettingsConfig, v: String| c.input.mod_key = v) as fn(&mut SettingsConfig, String),
        ),
        (
            "mod-key-nested",
            "mod key to use inside nested niri (e.g. Alt)",
            (|c: &SettingsConfig| c.input.mod_key_nested.as_str()) as fn(&SettingsConfig) -> &str,
            (|c: &mut SettingsConfig, v: String| c.input.mod_key_nested = v) as fn(&mut SettingsConfig, String),
        ),
    ] {
        let sep = Separator::new(Orientation::Horizontal);
        sep.add_css_class("row-sep");
        mk_card.append(&sep);

        let row = row_box();
        let lbl_col = label_col(label, hint);
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_text(getter(&cfg.borrow()));
        entry.set_placeholder_text(Some("Super"));
        entry.set_width_chars(14);
        entry.set_max_width_chars(30);
        let cfg_e = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            setter(&mut cfg_e.borrow_mut(), e.text().to_string());
        });
        row.append(&entry);
        mk_card.append(&row);
    }

    page.append(&mk_card);

    page.append(&save_button(cfg));
    page
}

// ── Layout page ───────────────────────────────────────────────────────────────

fn build_layout_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();

    page.append(&page_header("layout", "window layout & appearance"));

    // Card: spacing
    let sp_card = card("spacing");

    // Gaps slider row
    {
        let row = row_box();
        let lbl_col = label_col("gaps", "space between windows in logical pixels");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let val_lbl = Label::new(Some(&format!("{:.0}px", cfg.borrow().layout.gaps)));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_xalign(1.0);
        val_lbl.set_width_chars(5);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 64.0, 2.0);
        scale.set_value(cfg.borrow().layout.gaps);
        scale.set_width_request(120);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round();
            cfg_s.borrow_mut().layout.gaps = v;
            val_lbl_c.set_text(&format!("{v:.0}px"));
        });

        row.append(&scale);
        row.append(&val_lbl);
        sp_card.append(&row);
    }

    sp_card.append(&Separator::new(Orientation::Horizontal));

    // Center focused column
    {
        let row = row_box();
        let lbl = Label::new(Some("center focused column"));
        lbl.add_css_class("row-label");
        lbl.set_xalign(0.0);
        lbl.set_hexpand(true);
        row.append(&lbl);

        let current_idx = match cfg.borrow().layout.center_focused_column {
            CenterFocusedColumn::Never => 0u32,
            CenterFocusedColumn::OnOverflow => 1,
            CenterFocusedColumn::Always => 2,
        };
        let combo = DropDown::from_strings(&["never", "on overflow", "always"]);
        combo.set_selected(current_idx);

        let cfg_c = Rc::clone(&cfg);
        combo.connect_selected_notify(move |d| {
            let val = match d.selected() {
                1 => CenterFocusedColumn::OnOverflow,
                2 => CenterFocusedColumn::Always,
                _ => CenterFocusedColumn::Never,
            };
            cfg_c.borrow_mut().layout.center_focused_column = val;
        });

        row.append(&combo);
        sp_card.append(&row);
    }

    sp_card.append(&Separator::new(Orientation::Horizontal));

    append_toggle(
        &sp_card,
        "always center single column",
        "center a lone window regardless of the centering policy",
        cfg.borrow().layout.always_center_single_column,
        with_cfg(Rc::clone(&cfg), |c, v| {
            c.layout.always_center_single_column = v;
        }),
    );

    page.append(&sp_card);

    // Card: decoration
    let dec_card = card("decoration");

    append_color(
        &dec_card,
        "focus ring active color",
        &cfg.borrow().layout.focus_ring_active_color.clone(),
        with_cfg_str(Rc::clone(&cfg), |c, v| {
            c.layout.focus_ring_active_color = v;
        }),
    );

    dec_card.append(&Separator::new(Orientation::Horizontal));

    append_color(
        &dec_card,
        "focus ring inactive color",
        &cfg.borrow().layout.focus_ring_inactive_color.clone(),
        with_cfg_str(Rc::clone(&cfg), |c, v| {
            c.layout.focus_ring_inactive_color = v;
        }),
    );

    dec_card.append(&Separator::new(Orientation::Horizontal));

    append_toggle(
        &dec_card,
        "border",
        "draw borders around all windows (affects window sizing)",
        cfg.borrow().layout.border_on,
        with_cfg(Rc::clone(&cfg), |c, v| c.layout.border_on = v),
    );

    dec_card.append(&Separator::new(Orientation::Horizontal));

    append_toggle(
        &dec_card,
        "shadow",
        "draw drop shadows behind windows",
        cfg.borrow().layout.shadow_on,
        with_cfg(Rc::clone(&cfg), |c, v| c.layout.shadow_on = v),
    );

    page.append(&dec_card);
    page.append(&save_button(cfg));
    page
}

// ── Keybindings page ──────────────────────────────────────────────────────────

fn build_keybindings_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "keybindings",
        "define key bindings written to the binds {} block",
    ));

    // Wrap the dynamic list in a Rc<GtkBox> so rebuild() can clear+repopulate it.
    let list: Rc<GtkBox> = {
        let b = GtkBox::new(Orientation::Vertical, 8);
        b.set_margin_bottom(8);
        Rc::new(b)
    };

    fn rebuild(list: &Rc<GtkBox>, cfg: &Rc<RefCell<SettingsConfig>>) {
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        let binds = cfg.borrow().binds.clone();
        for (idx, bind) in binds.iter().enumerate() {
            let title = if !bind.key.is_empty() {
                format!("bind {} \u{2014} {}", idx + 1, bind.key)
            } else {
                format!("bind {}", idx + 1)
            };
            let c = card(&title);

            // ── key combo ─────────────────────────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "key combo",
                    "modifiers+key  e.g. Mod+T, XF86AudioMute, Super+Alt+L",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&bind.key);
                entry.set_placeholder_text(Some("Mod+T"));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                let list_c = Rc::clone(list);
                entry.connect_changed(move |e| {
                    cfg_c.borrow_mut().binds[idx].key = e.text().to_string();
                    // Update card title in place — rebuild would reset focus.
                    // Just store; title refresh happens on next full rebuild.
                    let _ = &list_c; // keep list alive
                });
                row.append(&entry);
                c.append(&row);
            }

            // ── action ────────────────────────────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("action", "what happens when the key is pressed");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(NIRI_ACTIONS);
                dd.set_enable_search(true);
                let sel_idx = NIRI_ACTIONS
                    .iter()
                    .position(|a| *a == bind.action.as_str())
                    .unwrap_or(0) as u32;
                dd.set_selected(sel_idx);
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    let name = NIRI_ACTIONS
                        .get(d.selected() as usize)
                        .copied()
                        .unwrap_or("spawn");
                    cfg_c.borrow_mut().binds[idx].action = name.to_string();
                });
                row.append(&dd);
                c.append(&row);
            }

            // ── action arguments ──────────────────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let hint = action_args_hint(&bind.action);
                let desc = if hint.is_empty() {
                    "arguments for the chosen action (leave blank if none)"
                } else {
                    "arguments for the chosen action"
                };
                let lc = label_col("args", desc);
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&bind.action_args);
                if !hint.is_empty() {
                    entry.set_placeholder_text(Some(hint));
                }
                entry.set_sensitive(action_needs_args(&bind.action) || !bind.action_args.is_empty());
                entry.set_width_chars(24);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    cfg_c.borrow_mut().binds[idx].action_args = e.text().to_string();
                });
                row.append(&entry);
                c.append(&row);
            }

            // ── options row: repeat + cooldown ────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();

                // repeat switch
                let lc_repeat = label_col("repeat", "fire repeatedly while the key is held");
                lc_repeat.set_hexpand(true);
                row.append(&lc_repeat);
                let sw_repeat = Switch::new();
                sw_repeat.set_active(bind.repeat);
                let cfg_c = Rc::clone(cfg);
                sw_repeat.connect_active_notify(move |s| {
                    cfg_c.borrow_mut().binds[idx].repeat = s.is_active();
                });
                row.append(&sw_repeat);

                // cooldown entry
                let lc_cd = label_col(
                    "cooldown-ms",
                    "min milliseconds between fires  (0 = disabled)",
                );
                lc_cd.set_margin_start(16);
                row.append(&lc_cd);
                let cd_entry = Entry::new();
                cd_entry.add_css_class("color-entry");
                cd_entry.set_width_chars(6);
                cd_entry.set_placeholder_text(Some("0"));
                if bind.cooldown_ms > 0 {
                    cd_entry.set_text(&bind.cooldown_ms.to_string());
                }
                let cfg_c = Rc::clone(cfg);
                cd_entry.connect_changed(move |e| {
                    let v: u32 = e.text().parse().unwrap_or(0);
                    cfg_c.borrow_mut().binds[idx].cooldown_ms = v;
                });
                row.append(&cd_entry);

                c.append(&row);
            }

            // ── allow-when-locked (spawn only) ────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "allow when locked",
                    "fire even while the screen is locked (spawn only)",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let sw = Switch::new();
                sw.set_active(bind.allow_when_locked);
                sw.set_sensitive(bind.action == "spawn");
                let cfg_c = Rc::clone(cfg);
                sw.connect_active_notify(move |s| {
                    cfg_c.borrow_mut().binds[idx].allow_when_locked = s.is_active();
                });
                row.append(&sw);
                c.append(&row);
            }

            // ── delete button ─────────────────────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let btn = Button::with_label("delete bind");
                btn.add_css_class("destructive-action");
                let cfg_c = Rc::clone(cfg);
                let list_c = Rc::clone(list);
                btn.connect_clicked(move |_| {
                    cfg_c.borrow_mut().binds.remove(idx);
                    rebuild(&list_c, &cfg_c);
                });
                row.append(&btn);
                c.append(&row);
            }

            list.append(&c);
        }
    }

    rebuild(&list, &cfg);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let inner = GtkBox::new(Orientation::Vertical, 8);
    inner.set_margin_start(0);
    inner.set_margin_end(0);
    inner.append(&*list);

    // ── add bind button ───────────────────────────────────────────────────────
    let add_btn = Button::with_label("+ add bind");
    add_btn.add_css_class("suggested-action-custom");
    add_btn.set_halign(Align::Start);
    let cfg_c = Rc::clone(&cfg);
    let list_c = Rc::clone(&list);
    add_btn.connect_clicked(move |_| {
        cfg_c.borrow_mut().binds.push(Keybind::default());
        rebuild(&list_c, &cfg_c);
    });

    // ── import from niri config button ────────────────────────────────────────
    let import_btn = Button::with_label("import from niri config");
    import_btn.add_css_class("suggested-action-custom");
    import_btn.set_halign(Align::Start);
    let cfg_c = Rc::clone(&cfg);
    let list_c = Rc::clone(&list);
    import_btn.connect_clicked(move |_| {
        let imported = import_binds_from_niri_config();
        if imported.is_empty() {
            log::warn!("settings: no binds found in niri config");
            return;
        }
        // Append imported binds, skipping duplicates by key.
        // Keep the set live so duplicates within the imported list are also
        // rejected (prevents double-insert when niri config has duplicate keys).
        let mut guard = cfg_c.borrow_mut();
        let mut seen_keys: std::collections::HashSet<String> =
            guard.binds.iter().map(|b| b.key.clone()).collect();
        let mut added = 0usize;
        for b in imported {
            if seen_keys.insert(b.key.clone()) {
                guard.binds.push(b);
                added += 1;
            }
        }
        drop(guard);
        log::info!("settings: imported {added} new binds from niri config");
        rebuild(&list_c, &cfg_c);
    });

    let btn_row = GtkBox::new(Orientation::Horizontal, 8);
    btn_row.append(&add_btn);
    btn_row.append(&import_btn);
    inner.append(&btn_row);

    scroll.set_child(Some(&inner));
    page.append(&scroll);
    page
}

// ── Outputs page ──────────────────────────────────────────────────────────────

fn build_outputs_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("outputs", "per-display mode, scale, transform & position"));

    // Status label shown until IPC query returns
    let status_label = Label::new(Some("Querying connected outputs…"));
    status_label.add_css_class("row-sublabel");
    status_label.set_margin_start(18);
    status_label.set_margin_top(8);
    page.append(&status_label);

    // Container that will hold per-output cards once IPC returns
    let cards_box = GtkBox::new(Orientation::Vertical, 12);
    page.append(&cards_box);

    // Kick off a background thread to query niri IPC
    let cfg_outer = Rc::clone(&cfg);
    let page_clone = page.clone();
    let status_clone = status_label.clone();
    let cards_clone = cards_box.clone();

    glib::spawn_future_local(async move {
        let result = gio_run_blocking(crate::ipc::query_outputs).await;
        // Remove status label
        page_clone.remove(&status_clone);

        match result {
            Err(e) => {
                log::warn!("outputs IPC query failed: {e}");
                let err_lbl = Label::new(Some(&format!("Could not query niri: {e}")));
                err_lbl.add_css_class("row-sublabel");
                err_lbl.set_margin_start(18);
                cards_clone.append(&err_lbl);
            }
            Ok(detected) => {
                // Ensure an OutputConfig exists for every detected output
                {
                    let mut c = cfg_outer.borrow_mut();
                    for out in &detected {
                        if !c.outputs.iter().any(|o| o.name == out.name) {
                            c.outputs.push(OutputConfig::new(&out.name));
                        }
                    }
                }

                for ipc_out in detected {
                    // Find (or create) the persisted config index for this output
                    let out_idx = {
                        let c = cfg_outer.borrow();
                        c.outputs.iter().position(|o| o.name == ipc_out.name).unwrap_or(0)
                    };

                    let title = format!(
                        "{} — {} {}",
                        ipc_out.name, ipc_out.make, ipc_out.model
                    );
                    let out_card = card(&title);

                    // ── off toggle ────────────────────────────────────────────
                    let init_off = cfg_outer.borrow().outputs[out_idx].off;
                    append_toggle(
                        &out_card,
                        "disable output",
                        "turn this display off",
                        init_off,
                        with_cfg(Rc::clone(&cfg_outer), move |c, v| c.outputs[out_idx].off = v),
                    );
                    out_card.append(&Separator::new(Orientation::Horizontal));

                    // ── mode dropdown ─────────────────────────────────────────
                    {
                        let row = row_box();
                        let lbl_col = label_col("mode", "resolution and refresh rate");
                        lbl_col.set_hexpand(true);
                        row.append(&lbl_col);

                        // Build labels: "auto" first, then each mode
                        let mut mode_labels: Vec<String> = vec!["auto".to_string()];
                        for m in &ipc_out.modes {
                            let hz = m.refresh_rate as f64 / 1000.0;
                            let label = format!("{}×{}  {:.3} Hz{}", m.width, m.height, hz,
                                if m.is_preferred { " ✓" } else { "" });
                            mode_labels.push(label);
                        }
                        let mode_strs: Vec<&str> = mode_labels.iter().map(|s| s.as_str()).collect();
                        let dd = DropDown::from_strings(&mode_strs);
                        dd.add_css_class("mini-dropdown");

                        // Select the currently saved mode (try to match w/h/refresh)
                        let saved_w = cfg_outer.borrow().outputs[out_idx].mode_width;
                        let saved_h = cfg_outer.borrow().outputs[out_idx].mode_height;
                        let saved_r = cfg_outer.borrow().outputs[out_idx].mode_refresh_mhz;
                        let selected = if saved_w == 0 {
                            0u32
                        } else {
                            ipc_out.modes.iter().position(|m| {
                                m.width == saved_w
                                    && m.height == saved_h
                                    && (saved_r == 0 || m.refresh_rate == saved_r)
                            }).map(|i| i as u32 + 1)
                            .unwrap_or(0)
                        };
                        dd.set_selected(selected);

                        let modes_copy = ipc_out.modes.clone();
                        let cfg_dd = Rc::clone(&cfg_outer);
                        dd.connect_selected_notify(move |d| {
                            let sel = d.selected() as usize;
                            let mut c = cfg_dd.borrow_mut();
                            if sel == 0 {
                                c.outputs[out_idx].mode_width = 0;
                                c.outputs[out_idx].mode_height = 0;
                                c.outputs[out_idx].mode_refresh_mhz = 0;
                            } else if let Some(m) = modes_copy.get(sel - 1) {
                                c.outputs[out_idx].mode_width = m.width;
                                c.outputs[out_idx].mode_height = m.height;
                                c.outputs[out_idx].mode_refresh_mhz = m.refresh_rate;
                            }
                        });
                        row.append(&dd);
                        out_card.append(&row);
                    }
                    out_card.append(&Separator::new(Orientation::Horizontal));

                    // ── scale ─────────────────────────────────────────────────
                    {
                        let row = row_box();
                        let lbl_col = label_col("scale", "fractional scaling factor (0 = auto)");
                        lbl_col.set_hexpand(true);
                        row.append(&lbl_col);

                        let init_scale = cfg_outer.borrow().outputs[out_idx].scale;
                        let val_lbl = Label::new(Some(&if init_scale == 0.0 {
                            "auto".to_string()
                        } else {
                            format!("{init_scale:.2}×")
                        }));
                        val_lbl.add_css_class("row-sublabel");
                        val_lbl.set_width_chars(7);

                        let scale_w = Scale::with_range(Orientation::Horizontal, 0.0, 4.0, 0.25);
                        scale_w.set_value(init_scale);
                        scale_w.set_width_request(140);
                        scale_w.set_draw_value(false);
                        scale_w.set_hexpand(false);

                        let cfg_sc = Rc::clone(&cfg_outer);
                        let val_lbl_c = val_lbl.clone();
                        scale_w.connect_value_changed(move |s| {
                            let v = (s.value() * 4.0).round() / 4.0;
                            cfg_sc.borrow_mut().outputs[out_idx].scale = v;
                            val_lbl_c.set_text(&if v == 0.0 {
                                "auto".to_string()
                            } else {
                                format!("{v:.2}×")
                            });
                        });
                        row.append(&scale_w);
                        row.append(&val_lbl);
                        out_card.append(&row);
                    }
                    out_card.append(&Separator::new(Orientation::Horizontal));

                    // ── transform ─────────────────────────────────────────────
                    {
                        let row = row_box();
                        let lbl_col =
                            label_col("transform", "rotation / flip applied to the display");
                        lbl_col.set_hexpand(true);
                        row.append(&lbl_col);

                        let dd =
                            DropDown::from_strings(OutputTransform::label_variants());
                        dd.add_css_class("mini-dropdown");
                        dd.set_selected(
                            cfg_outer.borrow().outputs[out_idx].transform.to_index(),
                        );
                        let cfg_tr = Rc::clone(&cfg_outer);
                        dd.connect_selected_notify(move |d| {
                            cfg_tr.borrow_mut().outputs[out_idx].transform =
                                OutputTransform::from_index(d.selected());
                        });
                        row.append(&dd);
                        out_card.append(&row);
                    }
                    out_card.append(&Separator::new(Orientation::Horizontal));

                    // ── VRR ───────────────────────────────────────────────────
                    {
                        let vrr_note = if ipc_out.vrr_supported {
                            "variable refresh rate"
                        } else {
                            "variable refresh rate (not supported by this display)"
                        };
                        let row = row_box();
                        let lbl_col = label_col("variable refresh rate", vrr_note);
                        lbl_col.set_hexpand(true);
                        row.append(&lbl_col);

                        let dd = DropDown::from_strings(VrrMode::label_variants());
                        dd.add_css_class("mini-dropdown");
                        dd.set_selected(cfg_outer.borrow().outputs[out_idx].vrr.to_index());
                        dd.set_sensitive(ipc_out.vrr_supported);
                        let cfg_vrr = Rc::clone(&cfg_outer);
                        dd.connect_selected_notify(move |d| {
                            cfg_vrr.borrow_mut().outputs[out_idx].vrr =
                                VrrMode::from_index(d.selected());
                        });
                        row.append(&dd);
                        out_card.append(&row);
                    }
                    out_card.append(&Separator::new(Orientation::Horizontal));

                    // ── position ─────────────────────────────────────────────
                    {
                        let init_set = cfg_outer.borrow().outputs[out_idx].position_set;
                        append_toggle(
                            &out_card,
                            "override position",
                            "manually set this display's position in the global space",
                            init_set,
                            with_cfg(Rc::clone(&cfg_outer), move |c, v| {
                                c.outputs[out_idx].position_set = v
                            }),
                        );

                        let pos_row = row_box();
                        let x_lbl = Label::new(Some("x"));
                        x_lbl.add_css_class("row-sublabel");
                        pos_row.append(&x_lbl);

                        let x_entry = Entry::new();
                        x_entry.add_css_class("color-entry");
                        x_entry.set_width_chars(6);
                        x_entry.set_text(&cfg_outer.borrow().outputs[out_idx].position_x.to_string());
                        let cfg_px = Rc::clone(&cfg_outer);
                        x_entry.connect_changed(move |e| {
                            if let Ok(v) = e.text().parse::<i32>() {
                                cfg_px.borrow_mut().outputs[out_idx].position_x = v;
                            }
                        });
                        pos_row.append(&x_entry);

                        let y_lbl = Label::new(Some("y"));
                        y_lbl.add_css_class("row-sublabel");
                        y_lbl.set_margin_start(8);
                        pos_row.append(&y_lbl);

                        let y_entry = Entry::new();
                        y_entry.add_css_class("color-entry");
                        y_entry.set_width_chars(6);
                        y_entry.set_text(&cfg_outer.borrow().outputs[out_idx].position_y.to_string());
                        let cfg_py = Rc::clone(&cfg_outer);
                        y_entry.connect_changed(move |e| {
                            if let Ok(v) = e.text().parse::<i32>() {
                                cfg_py.borrow_mut().outputs[out_idx].position_y = v;
                            }
                        });
                        pos_row.append(&y_entry);
                        out_card.append(&pos_row);
                    }

                    cards_clone.append(&out_card);
                }

                // Save button at bottom
                let save_btn = save_button(cfg_outer);
                cards_clone.append(&save_btn);
            }
        }
    });

    page
}

/// Runs a blocking closure on a Tokio thread-pool thread and returns the result
/// to the GTK main thread via a oneshot channel awaited as a future.
async fn gio_run_blocking<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.await.expect("gio_run_blocking: sender dropped")
}

// ── Switch Events page ───────────────────────────────────────────────────────

fn build_switch_events_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    fn add_event_row<F>(
        c: &GtkBox,
        label: &str,
        sub: &str,
        initial: Vec<String>,
        cfg: Rc<RefCell<SettingsConfig>>,
        setter: F,
    ) where
        F: Fn(&mut SwitchEventsSettings, Vec<String>) + 'static,
    {
        c.append(&Separator::new(Orientation::Horizontal));
        let row = row_box();
        let lc = label_col(label, sub);
        lc.set_hexpand(true);
        row.append(&lc);
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_text(&initial.join(" "));
        entry.set_placeholder_text(Some("program arg1 arg2 \u{2026}"));
        entry.set_width_chars(26);
        entry.connect_changed(move |e| {
            let argv: Vec<String> =
                e.text().to_string().split_whitespace().map(str::to_string).collect();
            setter(&mut cfg.borrow_mut().switch_events, argv);
        });
        row.append(&entry);
        c.append(&row);
    }

    let page = page_box();
    page.append(&page_header("switch events", "run commands when hardware switches change state"));

    let c = card("switch events");
    let hint = Label::new(Some(
        "Commands are space-separated argv  (e.g. notify-send Lid Closed).  \
         Leave blank to disable.",
    ));
    hint.set_wrap(true);
    hint.add_css_class("row-sublabel");
    hint.set_margin_start(14);
    hint.set_margin_end(14);
    hint.set_margin_top(4);
    hint.set_margin_bottom(8);
    c.append(&hint);

    let se = cfg.borrow().switch_events.clone();
    add_event_row(
        &c, "lid-close", "laptop lid closed",
        se.lid_close.clone(), Rc::clone(&cfg), |s, v| s.lid_close = v,
    );
    add_event_row(
        &c, "lid-open", "laptop lid opened",
        se.lid_open.clone(), Rc::clone(&cfg), |s, v| s.lid_open = v,
    );
    add_event_row(
        &c, "tablet-mode-on", "convertible enters tablet mode",
        se.tablet_mode_on.clone(), Rc::clone(&cfg), |s, v| s.tablet_mode_on = v,
    );
    add_event_row(
        &c, "tablet-mode-off", "convertible leaves tablet mode",
        se.tablet_mode_off.clone(), Rc::clone(&cfg), |s, v| s.tablet_mode_off = v,
    );

    page.append(&c);
    page.append(&save_button(cfg));
    page
}

// ── Layer Rules page ──────────────────────────────────────────────────────────

fn build_layer_rules_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    fn rebuild(rb: &Rc<GtkBox>, cfg: &Rc<RefCell<SettingsConfig>>) {
        while let Some(child) = rb.first_child() {
            rb.remove(&child);
        }
        let rules = cfg.borrow().layer_rules.clone();
        for (idx, rule) in rules.iter().enumerate() {
            let title = if !rule.match_namespace.is_empty() {
                format!("rule {} \u{2014} {}", idx + 1, rule.match_namespace)
            } else {
                format!("rule {}", idx + 1)
            };
            let c = card(&title);

            // namespace
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("namespace", "regex matching layer surface namespace (empty = any)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&rule.match_namespace);
                entry.set_placeholder_text(Some("e.g. ^waybar$"));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    cfg_c.borrow_mut().layer_rules[idx].match_namespace = e.text().to_string();
                });
                row.append(&entry);
                c.append(&row);
            }

            // at-startup
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("at startup", "match only during first 60 s after niri starts");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.match_at_startup.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().layer_rules[idx].match_at_startup =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }

            // opacity
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("opacity", "0.0\u{2013}1.0  (empty = no override)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.opacity != 0.0 {
                    entry.set_text(&format!("{:.2}", rule.opacity));
                }
                entry.set_placeholder_text(Some("0.0\u{2013}1.0"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<f64>().unwrap_or(0.0);
                    cfg_c.borrow_mut().layer_rules[idx].opacity = v;
                });
                row.append(&entry);
                c.append(&row);
            }

            // block-out-from
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("block-out-from", "replace with black in screencasts");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(BlockOutFrom::label_variants());
                dd.set_selected(rule.block_out_from.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().layer_rules[idx].block_out_from =
                        BlockOutFrom::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }

            // shadow
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("shadow", "force shadow on / off for this surface");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.shadow.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().layer_rules[idx].shadow =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }

            // corner radius
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("corner radius", "rounds shadow corners  (0 = no override)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.geometry_corner_radius != 0.0 {
                    entry.set_text(&format!("{:.1}", rule.geometry_corner_radius));
                }
                entry.set_placeholder_text(Some("px"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<f64>().unwrap_or(0.0);
                    cfg_c.borrow_mut().layer_rules[idx].geometry_corner_radius = v;
                });
                row.append(&entry);
                c.append(&row);
            }

            // place within backdrop
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("place within backdrop",
                    "show surface inside Overview / workspace-switch backdrop");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.place_within_backdrop.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().layer_rules[idx].place_within_backdrop =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }

            // delete button
            c.append(&Separator::new(Orientation::Horizontal));
            let del_row = row_box();
            let del_btn = Button::with_label("delete rule");
            del_btn.add_css_class("sidebar-item");
            del_btn.set_halign(Align::End);
            del_btn.set_hexpand(true);
            del_btn.set_margin_end(6);
            del_btn.set_margin_top(4);
            del_btn.set_margin_bottom(4);
            let cfg_d = Rc::clone(cfg);
            let rb_d = Rc::clone(rb);
            del_btn.connect_clicked(move |_| {
                cfg_d.borrow_mut().layer_rules.remove(idx);
                if let Err(e) = crate::settings_backend::save(&cfg_d.borrow()) {
                    log::warn!("settings: layer-rule delete save failed: {e}");
                }
                rebuild(&rb_d, &cfg_d);
            });
            del_row.append(&del_btn);
            c.append(&del_row);

            rb.append(&c);
        }
    }

    let page = page_box();
    page.append(&page_header("layer rules", "override properties for layer-shell surfaces"));

    let rules_box = Rc::new(GtkBox::new(Orientation::Vertical, 8));
    page.append(&*rules_box);
    rebuild(&rules_box, &cfg);

    let add_btn = Button::with_label("+ add rule");
    add_btn.add_css_class("save-btn");
    add_btn.set_halign(Align::Start);
    add_btn.set_margin_start(14);
    add_btn.set_margin_top(8);
    add_btn.set_margin_bottom(8);
    let cfg_a = Rc::clone(&cfg);
    let rb_a = Rc::clone(&rules_box);
    add_btn.connect_clicked(move |_| {
        cfg_a.borrow_mut().layer_rules.push(LayerRule::default());
        rebuild(&rb_a, &cfg_a);
    });
    page.append(&add_btn);
    page.append(&save_button(cfg));
    page
}

// ── Window Rules page ─────────────────────────────────────────────────────────

fn build_window_rules_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    fn rebuild(rb: &Rc<GtkBox>, cfg: &Rc<RefCell<SettingsConfig>>) {
        while let Some(child) = rb.first_child() {
            rb.remove(&child);
        }
        let rules = cfg.borrow().window_rules.clone();
        for (idx, rule) in rules.iter().enumerate() {
            let title = if !rule.match_app_id.is_empty() {
                format!("rule {} \u{2014} {}", idx + 1, rule.match_app_id)
            } else {
                format!("rule {}", idx + 1)
            };
            let c = card(&title);

            // ── match criteria ────────────────────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("app-id", "regex against window app ID (empty = any)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&rule.match_app_id);
                entry.set_placeholder_text(Some("e.g. ^firefox$"));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    cfg_c.borrow_mut().window_rules[idx].match_app_id = e.text().to_string();
                });
                row.append(&entry);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("title", "regex against window title (empty = any)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&rule.match_title);
                entry.set_placeholder_text(Some("e.g. ^Media viewer$"));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    cfg_c.borrow_mut().window_rules[idx].match_title = e.text().to_string();
                });
                row.append(&entry);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("at startup", "match only during first 60 s after niri starts");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.match_at_startup.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].match_at_startup =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }

            // ── opening properties ────────────────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("open maximized", "column fills monitor width on open");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.open_maximized.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].open_maximized =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("open fullscreen", "window opens in fullscreen mode");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.open_fullscreen.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].open_fullscreen =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("open floating", "window opens in the floating layout");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.open_floating.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].open_floating =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("open focused", "window receives keyboard focus when opened");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.open_focused.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].open_focused =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("open on output", "connector name e.g. HDMI-A-1  (empty = default)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&rule.open_on_output);
                entry.set_placeholder_text(Some("connector or make/model"));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    cfg_c.borrow_mut().window_rules[idx].open_on_output = e.text().to_string();
                });
                row.append(&entry);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("open on workspace", "named workspace  (empty = default)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&rule.open_on_workspace);
                entry.set_placeholder_text(Some("workspace name"));
                entry.set_width_chars(16);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    cfg_c.borrow_mut().window_rules[idx].open_on_workspace = e.text().to_string();
                });
                row.append(&entry);
                c.append(&row);
            }

            // ── dynamic properties ────────────────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("opacity", "0.0\u{2013}1.0  (empty = no override)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.opacity != 0.0 {
                    entry.set_text(&format!("{:.2}", rule.opacity));
                }
                entry.set_placeholder_text(Some("0.0\u{2013}1.0"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<f64>().unwrap_or(0.0);
                    cfg_c.borrow_mut().window_rules[idx].opacity = v;
                });
                row.append(&entry);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("block-out-from", "replace with black in screencasts");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(BlockOutFrom::label_variants());
                dd.set_selected(rule.block_out_from.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].block_out_from =
                        BlockOutFrom::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "draw border with background",
                    "draw border/focus-ring as filled rectangle",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.draw_border_with_background.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].draw_border_with_background =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "clip to geometry",
                    "clip window to visual geometry (rounds corners)",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.clip_to_geometry.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].clip_to_geometry =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "variable refresh rate",
                    "enable VRR on output when this window is displayed",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(rule.variable_refresh_rate.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    cfg_c.borrow_mut().window_rules[idx].variable_refresh_rate =
                        TriState::from_index(d.selected());
                });
                row.append(&dd);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("corner radius", "window geometry corner radius  (0 = no override)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.geometry_corner_radius != 0.0 {
                    entry.set_text(&format!("{:.1}", rule.geometry_corner_radius));
                }
                entry.set_placeholder_text(Some("px"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<f64>().unwrap_or(0.0);
                    cfg_c.borrow_mut().window_rules[idx].geometry_corner_radius = v;
                });
                row.append(&entry);
                c.append(&row);
            }

            // ── size limits ───────────────────────────────────────────────────
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("min / max width", "logical pixels  (0 = no limit)");
                lc.set_hexpand(true);
                row.append(&lc);
                let min_w = Entry::new();
                min_w.add_css_class("color-entry");
                if rule.min_width != 0 { min_w.set_text(&rule.min_width.to_string()); }
                min_w.set_placeholder_text(Some("min px"));
                min_w.set_width_chars(7);
                let cfg_c = Rc::clone(cfg);
                min_w.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<u32>().unwrap_or(0);
                    cfg_c.borrow_mut().window_rules[idx].min_width = v;
                });
                row.append(&min_w);
                let max_w = Entry::new();
                max_w.add_css_class("color-entry");
                if rule.max_width != 0 { max_w.set_text(&rule.max_width.to_string()); }
                max_w.set_placeholder_text(Some("max px"));
                max_w.set_width_chars(7);
                let cfg_c = Rc::clone(cfg);
                max_w.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<u32>().unwrap_or(0);
                    cfg_c.borrow_mut().window_rules[idx].max_width = v;
                });
                row.append(&max_w);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("min / max height", "logical pixels  (0 = no limit)");
                lc.set_hexpand(true);
                row.append(&lc);
                let min_h = Entry::new();
                min_h.add_css_class("color-entry");
                if rule.min_height != 0 { min_h.set_text(&rule.min_height.to_string()); }
                min_h.set_placeholder_text(Some("min px"));
                min_h.set_width_chars(7);
                let cfg_c = Rc::clone(cfg);
                min_h.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<u32>().unwrap_or(0);
                    cfg_c.borrow_mut().window_rules[idx].min_height = v;
                });
                row.append(&min_h);
                let max_h = Entry::new();
                max_h.add_css_class("color-entry");
                if rule.max_height != 0 { max_h.set_text(&rule.max_height.to_string()); }
                max_h.set_placeholder_text(Some("max px"));
                max_h.set_width_chars(7);
                let cfg_c = Rc::clone(cfg);
                max_h.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<u32>().unwrap_or(0);
                    cfg_c.borrow_mut().window_rules[idx].max_height = v;
                });
                row.append(&max_h);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("scroll factor", "multiplies all scroll events  (0 = no override)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.scroll_factor != 0.0 {
                    entry.set_text(&format!("{:.2}", rule.scroll_factor));
                }
                entry.set_placeholder_text(Some("e.g. 0.75"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    let v = e.text().to_string().parse::<f64>().unwrap_or(0.0);
                    cfg_c.borrow_mut().window_rules[idx].scroll_factor = v;
                });
                row.append(&entry);
                c.append(&row);
            }

            // delete button
            c.append(&Separator::new(Orientation::Horizontal));
            let del_row = row_box();
            let del_btn = Button::with_label("delete rule");
            del_btn.add_css_class("sidebar-item");
            del_btn.set_halign(Align::End);
            del_btn.set_hexpand(true);
            del_btn.set_margin_end(6);
            del_btn.set_margin_top(4);
            del_btn.set_margin_bottom(4);
            let cfg_d = Rc::clone(cfg);
            let rb_d = Rc::clone(rb);
            del_btn.connect_clicked(move |_| {
                cfg_d.borrow_mut().window_rules.remove(idx);
                if let Err(e) = crate::settings_backend::save(&cfg_d.borrow()) {
                    log::warn!("settings: window-rule delete save failed: {e}");
                }
                rebuild(&rb_d, &cfg_d);
            });
            del_row.append(&del_btn);
            c.append(&del_row);

            rb.append(&c);
        }
    }

    let page = page_box();
    page.append(&page_header("window rules", "override properties per window"));

    let rules_box = Rc::new(GtkBox::new(Orientation::Vertical, 8));
    page.append(&*rules_box);
    rebuild(&rules_box, &cfg);

    let add_btn = Button::with_label("+ add rule");
    add_btn.add_css_class("save-btn");
    add_btn.set_halign(Align::Start);
    add_btn.set_margin_start(14);
    add_btn.set_margin_top(8);
    add_btn.set_margin_bottom(8);
    let cfg_a = Rc::clone(&cfg);
    let rb_a = Rc::clone(&rules_box);
    add_btn.connect_clicked(move |_| {
        cfg_a.borrow_mut().window_rules.push(WindowRule::default());
        rebuild(&rb_a, &cfg_a);
    });
    page.append(&add_btn);
    page.append(&save_button(cfg));
    page
}

// ── Generic stub page ─────────────────────────────────────────────────────────

fn build_stub_page(name: &str, message: &str) -> GtkBox {
    let page = page_box();
    page.append(&page_header(name, ""));
    let c = card(name);
    let lbl = Label::new(Some(message));
    lbl.add_css_class("stub-label");
    lbl.set_xalign(0.0);
    lbl.set_wrap(true);
    c.append(&lbl);
    page.append(&c);
    page
}

// ── Animations page ───────────────────────────────────────────────────────────

fn build_animations_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("animations", "enable, disable & slow down niri animations"));

    // Card: global
    let global_card = card("global");
    append_toggle(
        &global_card,
        "disable all animations",
        "turn off every animation at once (overrides individual settings below)",
        cfg.borrow().anim.global_off,
        with_cfg(Rc::clone(&cfg), |c, v| c.anim.global_off = v),
    );
    global_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lbl_col = label_col(
            "slowdown factor",
            "multiply all animation durations  (0 = niri default; < 1 = faster)",
        );
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init = cfg.borrow().anim.slowdown;
        let val_lbl = Label::new(Some(&if init == 0.0 {
            "default".to_string()
        } else {
            format!("{init:.1}×")
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 10.0, 0.1);
        scale.set_value(init);
        scale.set_width_request(150);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 10.0).round() / 10.0;
            cfg_s.borrow_mut().anim.slowdown = v;
            val_lbl_c.set_text(&if v == 0.0 {
                "default".to_string()
            } else {
                format!("{v:.1}×")
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        global_card.append(&row);
    }
    page.append(&global_card);

    // Card: per-animation off toggles
    let anim_card = card("individual animations");
    let anim_toggles: &[AnimBoolRow] = &[
        ("workspace-switch", "switching workspaces up/down",
            |a| a.workspace_switch_off, |a, v| a.workspace_switch_off = v),
        ("window-open", "window appearing on screen",
            |a| a.window_open_off, |a, v| a.window_open_off = v),
        ("window-close", "window disappearing",
            |a| a.window_close_off, |a, v| a.window_close_off = v),
        ("horizontal-view-movement", "camera scrolling left/right",
            |a| a.horizontal_view_movement_off, |a, v| a.horizontal_view_movement_off = v),
        ("window-movement", "windows moving within a workspace",
            |a| a.window_movement_off, |a, v| a.window_movement_off = v),
        ("window-resize", "window size changes",
            |a| a.window_resize_off, |a, v| a.window_resize_off = v),
        ("config-notification-open-close", "config error notification slide",
            |a| a.config_notification_open_close_off, |a, v| a.config_notification_open_close_off = v),
        ("exit-confirmation-open-close", "exit confirmation dialog",
            |a| a.exit_confirmation_open_close_off, |a, v| a.exit_confirmation_open_close_off = v),
        ("screenshot-ui-open", "screenshot picker fade-in",
            |a| a.screenshot_ui_open_off, |a, v| a.screenshot_ui_open_off = v),
        ("overview-open-close", "overview zoom in/out",
            |a| a.overview_open_close_off, |a, v| a.overview_open_close_off = v),
        ("recent-windows-close", "recent windows switcher fade-out",
            |a| a.recent_windows_close_off, |a, v| a.recent_windows_close_off = v),
    ];

    let mut first = true;
    for (name, desc, getter, setter) in anim_toggles {
        if !first {
            anim_card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;

        let label = format!("disable {name}");
        let init = getter(&cfg.borrow().anim);
        let setter_copy = *setter;
        append_toggle(
            &anim_card,
            &label,
            desc,
            init,
            with_cfg(Rc::clone(&cfg), move |c, v| setter_copy(&mut c.anim, v)),
        );
    }
    page.append(&anim_card);

    page.append(&save_button(cfg));
    page
}

// ── Workspaces page ───────────────────────────────────────────────────────────

fn build_workspaces_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    use crate::settings_backend::NamedWorkspace;

    let page = page_box();
    page.append(&page_header("workspaces", "named workspaces that always exist"));

    let list_card = Rc::new(card("named workspaces"));
    page.append(&*list_card);

    // Rebuild helper — clears and repopulates the card rows
    fn rebuild(card: &GtkBox, cfg: &Rc<RefCell<SettingsConfig>>) {
        // Remove all children except the title (first child)
        let mut to_remove: Vec<gtk4::Widget> = Vec::new();
        let mut child = card.first_child();
        // skip first child (card-title label)
        if let Some(ref w) = child {
            child = w.next_sibling();
        }
        while let Some(w) = child {
            to_remove.push(w.clone());
            child = w.next_sibling();
        }
        for w in to_remove {
            card.remove(&w);
        }

        let workspaces = cfg.borrow().workspaces.clone();
        for (idx, ws) in workspaces.iter().enumerate() {
            let sep = Separator::new(Orientation::Horizontal);
            sep.add_css_class("row-sep");
            card.append(&sep);

            let row = row_box();

            // Name entry
            let name_entry = Entry::new();
            name_entry.add_css_class("color-entry");
            name_entry.set_text(&ws.name);
            name_entry.set_placeholder_text(Some("workspace name"));
            name_entry.set_width_chars(14);
            name_entry.set_hexpand(true);
            let cfg_n = Rc::clone(cfg);
            name_entry.connect_changed(move |e| {
                cfg_n.borrow_mut().workspaces[idx].name = e.text().to_string();
            });
            row.append(&name_entry);

            // Output entry
            let out_lbl = Label::new(Some("on output:"));
            out_lbl.add_css_class("row-sublabel");
            out_lbl.set_margin_start(8);
            row.append(&out_lbl);

            let out_entry = Entry::new();
            out_entry.add_css_class("color-entry");
            out_entry.set_text(&ws.open_on_output);
            out_entry.set_placeholder_text(Some("any"));
            out_entry.set_width_chars(12);
            let cfg_o = Rc::clone(cfg);
            out_entry.connect_changed(move |e| {
                cfg_o.borrow_mut().workspaces[idx].open_on_output = e.text().to_string();
            });
            row.append(&out_entry);

            // Delete button
            let del_btn = Button::with_label("✕");
            del_btn.add_css_class("sidebar-item");
            let cfg_d = Rc::clone(cfg);
            del_btn.connect_clicked(move |btn| {
                cfg_d.borrow_mut().workspaces.remove(idx);
                // Trigger save so row disappears
                if let Err(e) = crate::settings_backend::save(&cfg_d.borrow()) {
                    log::warn!("settings: failed to save after workspace delete: {e}");
                }
                // Find parent GtkBox (the card) and parent page, then rebuild
                if let Some(card_w) = btn.parent().and_then(|r| r.parent()) {
                    if let Ok(card_box) = card_w.downcast::<GtkBox>() {
                        rebuild(&card_box, &cfg_d);
                    }
                }
            });
            row.append(&del_btn);
            card.append(&row);
        }
    }

    rebuild(&list_card, &cfg);

    // Add workspace button
    let add_btn = Button::with_label("+ add workspace");
    add_btn.add_css_class("save-btn");
    add_btn.set_halign(Align::Start);
    add_btn.set_margin_start(14);
    add_btn.set_margin_top(8);
    add_btn.set_margin_bottom(8);
    let cfg_a = Rc::clone(&cfg);
    let list_card_c = Rc::clone(&list_card);
    add_btn.connect_clicked(move |_| {
        cfg_a.borrow_mut().workspaces.push(NamedWorkspace {
            name: String::new(),
            open_on_output: String::new(),
        });
        rebuild(&list_card_c, &cfg_a);
    });
    page.append(&add_btn);

    page.append(&save_button(cfg));
    page
}

// ── Gestures page ─────────────────────────────────────────────────────────────

fn build_gestures_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("gestures", "drag-and-drop scrolling & hot corners"));

    // Card: dnd-edge-view-scroll
    let vs_card = card("dnd edge view scroll");

    let vs_rows: &[CfgU32Row] = &[
        ("trigger width", "px from edge that activates scrolling (0=default 30)",
            3000, |c| c.gestures.dnd_view_trigger_width, |c, v| c.gestures.dnd_view_trigger_width = v),
        ("delay", "ms before scrolling starts (0=default 100)",
            2000, |c| c.gestures.dnd_view_delay_ms, |c, v| c.gestures.dnd_view_delay_ms = v),
        ("max speed", "px/s maximum scroll speed (0=default 1500)",
            5000, |c| c.gestures.dnd_view_max_speed, |c, v| c.gestures.dnd_view_max_speed = v),
    ];

    append_u32_sliders(&vs_card, vs_rows, Rc::clone(&cfg));
    page.append(&vs_card);

    // Card: dnd-edge-workspace-switch
    let ws_card = card("dnd edge workspace switch");

    let ws_rows: &[CfgU32Row] = &[
        ("trigger height", "px from edge that activates switching (0=default 50)",
            3000, |c| c.gestures.dnd_ws_trigger_height, |c, v| c.gestures.dnd_ws_trigger_height = v),
        ("delay", "ms before switching starts (0=default 100)",
            2000, |c| c.gestures.dnd_ws_delay_ms, |c, v| c.gestures.dnd_ws_delay_ms = v),
        ("max speed", "maximum switching speed (0=default 1500)",
            5000, |c| c.gestures.dnd_ws_max_speed, |c, v| c.gestures.dnd_ws_max_speed = v),
    ];

    append_u32_sliders(&ws_card, ws_rows, Rc::clone(&cfg));
    page.append(&ws_card);

    // Card: hot corners
    let hc_card = card("hot corners");
    append_toggle(
        &hc_card,
        "disable hot corners",
        "turn off the corner trigger for the overview",
        cfg.borrow().gestures.hot_corners_off,
        with_cfg(Rc::clone(&cfg), |c, v| c.gestures.hot_corners_off = v),
    );
    hc_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &hc_card,
        "top-left",
        "top-left corner triggers overview",
        cfg.borrow().gestures.hot_corners_top_left,
        with_cfg(Rc::clone(&cfg), |c, v| c.gestures.hot_corners_top_left = v),
    );
    hc_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &hc_card,
        "top-right",
        "top-right corner triggers overview",
        cfg.borrow().gestures.hot_corners_top_right,
        with_cfg(Rc::clone(&cfg), |c, v| c.gestures.hot_corners_top_right = v),
    );
    hc_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &hc_card,
        "bottom-left",
        "bottom-left corner triggers overview",
        cfg.borrow().gestures.hot_corners_bottom_left,
        with_cfg(Rc::clone(&cfg), |c, v| c.gestures.hot_corners_bottom_left = v),
    );
    hc_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &hc_card,
        "bottom-right",
        "bottom-right corner triggers overview",
        cfg.borrow().gestures.hot_corners_bottom_right,
        with_cfg(Rc::clone(&cfg), |c, v| c.gestures.hot_corners_bottom_right = v),
    );
    page.append(&hc_card);

    page.append(&save_button(cfg));
    page
}

/// Helper: appends a sequence of u32 slider rows to a card.
type U32Row = (&'static str, &'static str, u32, fn(&SettingsConfig) -> u32, fn(&mut SettingsConfig, u32));

fn append_u32_sliders(card: &GtkBox, rows: &[U32Row], cfg: Rc<RefCell<SettingsConfig>>) {
    let mut first = true;
    for (label, desc, max, getter, setter) in rows {
        if !first {
            card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;

        let row = row_box();
        let lbl_col = label_col(label, desc);
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init = getter(&cfg.borrow());
        let val_lbl = Label::new(Some(&if init == 0 {
            "default".to_string()
        } else {
            init.to_string()
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(7);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, *max as f64, 1.0);
        scale.set_value(init as f64);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        let setter_copy = *setter;
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            setter_copy(&mut cfg_s.borrow_mut(), v);
            val_lbl_c.set_text(&if v == 0 {
                "default".to_string()
            } else {
                v.to_string()
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        card.append(&row);
    }
}

// ── Recent windows page ────────────────────────────────────────────────────────

fn build_recent_windows_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("recent windows", "Alt-Tab switcher configuration"));

    // Card: general
    let gen_card = card("general");
    append_toggle(
        &gen_card,
        "disable recent windows switcher",
        "turn off the Alt-Tab window switcher entirely",
        cfg.borrow().recent_windows.off,
        with_cfg(Rc::clone(&cfg), |c, v| c.recent_windows.off = v),
    );
    gen_card.append(&Separator::new(Orientation::Horizontal));

    let rw_rows: &[U32Row] = &[
        ("debounce delay", "ms before focusing a window commits it to the list (0=default 750)",
            2000, |c| c.recent_windows.debounce_ms, |c, v| c.recent_windows.debounce_ms = v),
        ("open delay", "ms between pressing Alt-Tab and the switcher appearing (0=default 150)",
            1000, |c| c.recent_windows.open_delay_ms, |c, v| c.recent_windows.open_delay_ms = v),
    ];
    append_u32_sliders(&gen_card, rw_rows, Rc::clone(&cfg));
    page.append(&gen_card);

    // Card: highlight
    let hl_card = card("highlight");
    {
        let init = cfg.borrow().recent_windows.highlight_active_color.clone();
        append_color(
            &hl_card,
            "active color",
            &init,
            with_cfg_str(Rc::clone(&cfg), |c, v| c.recent_windows.highlight_active_color = v),
        );
    }
    hl_card.append(&Separator::new(Orientation::Horizontal));
    {
        let init = cfg.borrow().recent_windows.highlight_urgent_color.clone();
        append_color(
            &hl_card,
            "urgent color",
            &init,
            with_cfg_str(Rc::clone(&cfg), |c, v| c.recent_windows.highlight_urgent_color = v),
        );
    }
    hl_card.append(&Separator::new(Orientation::Horizontal));

    let hl_rows: &[U32Row] = &[
        ("padding", "px of padding around the focused preview (0=default 30)",
            100, |c| c.recent_windows.highlight_padding, |c, v| c.recent_windows.highlight_padding = v),
        ("corner radius", "radius of the highlight corners in px",
            60, |c| c.recent_windows.highlight_corner_radius, |c, v| c.recent_windows.highlight_corner_radius = v),
    ];
    append_u32_sliders(&hl_card, hl_rows, Rc::clone(&cfg));
    page.append(&hl_card);

    // Card: previews
    let pv_card = card("previews");
    {
        let pv_max_height_row: &[U32Row] = &[
            ("max height", "maximum height of window previews in px (0=default 480)",
                1440, |c| c.recent_windows.previews_max_height, |c, v| c.recent_windows.previews_max_height = v),
        ];
        append_u32_sliders(&pv_card, pv_max_height_row, Rc::clone(&cfg));
    }
    pv_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lbl_col = label_col(
            "max scale",
            "maximum scale factor for preview windows (0.0=default 0.5)",
        );
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init = cfg.borrow().recent_windows.previews_max_scale;
        let val_lbl = Label::new(Some(&if init == 0.0 {
            "default".to_string()
        } else {
            format!("{init:.2}")
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        scale.set_value(init);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 100.0).round() / 100.0;
            cfg_s.borrow_mut().recent_windows.previews_max_scale = v;
            val_lbl_c.set_text(&if v == 0.0 {
                "default".to_string()
            } else {
                format!("{v:.2}")
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        pv_card.append(&row);
    }
    page.append(&pv_card);

    page.append(&save_button(cfg));
    page
}

// ── Debug options page ────────────────────────────────────────────────────────

fn build_debug_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "debug",
        "experimental & diagnostic flags — not covered by config compatibility policy",
    ));

    // Card: rendering
    let render_card = card("rendering");
    let render_toggles: &[CfgBoolRow] = &[
        ("enable overlay planes",
            "direct scanout into overlay planes (may cause frame drops on some hardware)",
            |c| c.debug.enable_overlay_planes, |c, v| c.debug.enable_overlay_planes = v),
        ("disable cursor plane",
            "render cursor with the rest of the frame instead of using a dedicated plane",
            |c| c.debug.disable_cursor_plane, |c, v| c.debug.disable_cursor_plane = v),
        ("disable direct scanout",
            "disable direct scanout to both primary and overlay planes",
            |c| c.debug.disable_direct_scanout, |c, v| c.debug.disable_direct_scanout = v),
        ("restrict primary scanout to matching format",
            "only scan out to primary plane when the buffer format exactly matches",
            |c| c.debug.restrict_primary_scanout_to_matching_format,
            |c, v| c.debug.restrict_primary_scanout_to_matching_format = v),
        ("force disable connectors on resume",
            "force a modeset/screen blank on all outputs when waking from suspend",
            |c| c.debug.force_disable_connectors_on_resume,
            |c, v| c.debug.force_disable_connectors_on_resume = v),
        ("force pipewire invalid modifier",
            "use the invalid DRM modifier for PipeWire screencasting",
            |c| c.debug.force_pipewire_invalid_modifier,
            |c, v| c.debug.force_pipewire_invalid_modifier = v),
        ("skip cursor-only updates during VRR",
            "skip redraws triggered only by cursor movement while VRR is active",
            |c| c.debug.skip_cursor_only_updates_during_vrr,
            |c, v| c.debug.skip_cursor_only_updates_during_vrr = v),
    ];

    let mut first = true;
    for (label, desc, getter, setter) in render_toggles {
        if !first {
            render_card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;
        let init = getter(&cfg.borrow());
        let setter_copy = *setter;
        append_toggle(
            &render_card,
            label,
            desc,
            init,
            with_cfg(Rc::clone(&cfg), setter_copy),
        );
    }

    // render-drm-device path entry
    render_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lbl_col = label_col("render DRM device", "override the DRM device used for rendering (empty = auto)");
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_text(&cfg.borrow().debug.render_drm_device);
        entry.set_placeholder_text(Some("/dev/dri/renderD129"));
        entry.set_width_chars(20);
        entry.set_max_width_chars(40);
        let cfg_e = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            cfg_e.borrow_mut().debug.render_drm_device = e.text().to_string();
        });
        row.append(&entry);
        render_card.append(&row);
    }
    page.append(&render_card);

    // Card: window management
    let wm_card = card("window management");
    let wm_toggles: &[CfgBoolRow] = &[
        ("disable resize throttling",
            "send resizes to windows as fast as possible (very fast on high-Hz mice)",
            |c| c.debug.disable_resize_throttling,
            |c, v| c.debug.disable_resize_throttling = v),
        ("disable transactions",
            "disable resize/close transactions (windows may resize unsynchronised)",
            |c| c.debug.disable_transactions, |c, v| c.debug.disable_transactions = v),
        ("strict new window focus policy",
            "only focus windows that activate with a valid xdg-activation token",
            |c| c.debug.strict_new_window_focus_policy,
            |c, v| c.debug.strict_new_window_focus_policy = v),
        ("honor xdg-activation with invalid serial",
            "let apps like Discord/Telegram steal focus via tray/notification click",
            |c| c.debug.honor_xdg_activation_with_invalid_serial,
            |c, v| c.debug.honor_xdg_activation_with_invalid_serial = v),
        ("deactivate unfocused windows",
            "drop the Activated state for all unfocused windows (helps Electron apps)",
            |c| c.debug.deactivate_unfocused_windows,
            |c, v| c.debug.deactivate_unfocused_windows = v),
        ("keep laptop panel on when lid is closed",
            "leave the internal monitor powered on even when the lid is shut",
            |c| c.debug.keep_laptop_panel_on_when_lid_is_closed,
            |c, v| c.debug.keep_laptop_panel_on_when_lid_is_closed = v),
        ("disable monitor names",
            "ignore make/model/serial from EDID — workaround for a 0.1.9/0.1.10 crash",
            |c| c.debug.disable_monitor_names, |c, v| c.debug.disable_monitor_names = v),
    ];

    first = true;
    for (label, desc, getter, setter) in wm_toggles {
        if !first {
            wm_card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;
        let init = getter(&cfg.borrow());
        let setter_copy = *setter;
        append_toggle(
            &wm_card,
            label,
            desc,
            init,
            with_cfg(Rc::clone(&cfg), setter_copy),
        );
    }
    page.append(&wm_card);

    // Card: D-Bus / misc
    let misc_card = card("d-bus & diagnostics");
    let misc_toggles: &[CfgBoolRow] = &[
        ("D-Bus interfaces in non-session instances",
            "create D-Bus interfaces even when not running as --session (for testing)",
            |c| c.debug.dbus_interfaces_in_non_session_instances,
            |c, v| c.debug.dbus_interfaces_in_non_session_instances = v),
        ("wait for frame completion before queueing",
            "wait until every frame is done before handing to DRM",
            |c| c.debug.wait_for_frame_completion_before_queueing,
            |c, v| c.debug.wait_for_frame_completion_before_queueing = v),
        ("emulate zero presentation time",
            "emulate unknown DRM presentation time (NVIDIA proprietary behaviour)",
            |c| c.debug.emulate_zero_presentation_time,
            |c, v| c.debug.emulate_zero_presentation_time = v),
    ];

    first = true;
    for (label, desc, getter, setter) in misc_toggles {
        if !first {
            misc_card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;
        let init = getter(&cfg.borrow());
        let setter_copy = *setter;
        append_toggle(
            &misc_card,
            label,
            desc,
            init,
            with_cfg(Rc::clone(&cfg), setter_copy),
        );
    }
    page.append(&misc_card);

    page.append(&save_button(cfg));
    page
}

// ── Miscellaneous page ─────────────────────────────────────────────────────

fn build_miscellaneous_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("miscellaneous", "cursor, overview, screenshot, clipboard & more"));

    // ── SCREENSHOT ───────────────────────────────────────────────────────────
    let ss_card = card("screenshot");
    {
        let row = row_box();
        let lc = label_col(
            "save path",
            "strftime codes for date/time; empty = niri default; \"null\" = disable saving",
        );
        lc.set_hexpand(true);
        row.append(&lc);
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_width_chars(26);
        entry.set_max_width_chars(36);
        entry.set_placeholder_text(Some("~/Pictures/Screenshots/…"));
        entry.set_text(&cfg.borrow().misc.screenshot_path);
        let cfg_s = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            cfg_s.borrow_mut().misc.screenshot_path = e.text().to_string();
        });
        row.append(&entry);
        ss_card.append(&row);
    }
    page.append(&ss_card);

    // ── CURSOR ──────────────────────────────────────────────────────────────
    let cur_card = card("cursor");
    {
        let row = row_box();
        let lc = label_col("xcursor theme", "cursor theme name  (e.g. breeze_cursors)");
        lc.set_hexpand(true);
        row.append(&lc);
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_width_chars(16);
        entry.set_placeholder_text(Some("system default"));
        entry.set_text(&cfg.borrow().misc.cursor_theme);
        let cfg_s = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            cfg_s.borrow_mut().misc.cursor_theme = e.text().to_string();
        });
        row.append(&entry);
        cur_card.append(&row);
    }
    cur_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lc = label_col("xcursor size", "cursor size in pixels  (0 = system default)");
        lc.set_hexpand(true);
        row.append(&lc);
        let init = cfg.borrow().misc.cursor_size;
        let init_text = if init == 0 { String::from("default") } else { format!("{}px", init) };
        let val_lbl = Label::new(Some(&init_text));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(7);
        val_lbl.set_xalign(1.0);
        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 128.0, 1.0);
        scale.set_value(init as f64);
        scale.set_width_request(120);
        scale.set_draw_value(false);
        scale.set_hexpand(false);
        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            cfg_s.borrow_mut().misc.cursor_size = v;
            let t = if v == 0 { String::from("default") } else { format!("{}px", v) };
            val_lbl_c.set_text(&t);
        });
        row.append(&scale);
        row.append(&val_lbl);
        cur_card.append(&row);
    }
    cur_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &cur_card,
        "hide when typing",
        "hide the cursor while keys are being pressed",
        cfg.borrow().misc.cursor_hide_when_typing,
        with_cfg(Rc::clone(&cfg), |c, v| c.misc.cursor_hide_when_typing = v),
    );
    cur_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lc = label_col("hide after idle", "hide cursor after N ms with no movement  (0 = never)");
        lc.set_hexpand(true);
        row.append(&lc);
        let init_ms = cfg.borrow().misc.cursor_hide_after_inactive_ms;
        let val_lbl = Label::new(Some(&idle_ms_label(init_ms)));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);
        val_lbl.set_xalign(1.0);
        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 10_000.0, 500.0);
        scale.set_value(init_ms as f64);
        scale.set_width_request(120);
        scale.set_draw_value(false);
        scale.set_hexpand(false);
        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            cfg_s.borrow_mut().misc.cursor_hide_after_inactive_ms = v;
            val_lbl_c.set_text(&idle_ms_label(v));
        });
        row.append(&scale);
        row.append(&val_lbl);
        cur_card.append(&row);
    }
    page.append(&cur_card);

    // ── OVERVIEW ────────────────────────────────────────────────────────────
    let ov_card = card("overview");
    {
        let row = row_box();
        let lc = label_col("zoom", "how much workspaces shrink in the overview  (0 = niri default)");
        lc.set_hexpand(true);
        row.append(&lc);
        let init_zoom = cfg.borrow().misc.overview_zoom;
        let val_lbl = Label::new(Some(&zoom_label(init_zoom)));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);
        val_lbl.set_xalign(1.0);
        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 0.75, 0.05);
        scale.set_value(init_zoom);
        scale.set_width_request(120);
        scale.set_draw_value(false);
        scale.set_hexpand(false);
        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 20.0).round() / 20.0;
            cfg_s.borrow_mut().misc.overview_zoom = v;
            val_lbl_c.set_text(&zoom_label(v));
        });
        row.append(&scale);
        row.append(&val_lbl);
        ov_card.append(&row);
    }
    ov_card.append(&Separator::new(Orientation::Horizontal));
    {
        let init_color = cfg.borrow().misc.overview_backdrop_color.clone();
        append_color(
            &ov_card,
            "backdrop color",
            &init_color,
            with_cfg_str(Rc::clone(&cfg), |c, v| c.misc.overview_backdrop_color = v),
        );
    }
    page.append(&ov_card);

    // ── CLIPBOARD ──────────────────────────────────────────────────────────
    let clip_card = card("clipboard");
    append_toggle(
        &clip_card,
        "disable primary selection",
        "disable middle-click paste (primary clipboard)",
        cfg.borrow().misc.clipboard_disable_primary,
        with_cfg(Rc::clone(&cfg), |c, v| c.misc.clipboard_disable_primary = v),
    );
    page.append(&clip_card);

    // ── HOTKEY OVERLAY ───────────────────────────────────────────────
    let hk_card = card("hotkey overlay");
    append_toggle(
        &hk_card,
        "hide unbound actions",
        "only show hotkey overlay entries that are actually bound to a key",
        cfg.borrow().misc.hotkey_overlay_hide_not_bound,
        with_cfg(Rc::clone(&cfg), |c, v| c.misc.hotkey_overlay_hide_not_bound = v),
    );
    page.append(&hk_card);

    // ── CONFIG NOTIFICATIONS ──────────────────────────────────────────
    let notif_card = card("config notifications");
    append_toggle(
        &notif_card,
        "disable parse error notification",
        "don't show the 'Failed to parse config' desktop notification",
        cfg.borrow().misc.config_notification_disable_failed,
        with_cfg(Rc::clone(&cfg), |c, v| c.misc.config_notification_disable_failed = v),
    );
    page.append(&notif_card);

    // ── XWAYLAND ─────────────────────────────────────────────────────────────
    let xwl_card = card("xwayland");
    append_toggle(
        &xwl_card,
        "disable xwayland-satellite",
        "turn off automatic Xwayland X11 integration",
        cfg.borrow().misc.xwayland_off,
        with_cfg(Rc::clone(&cfg), |c, v| c.misc.xwayland_off = v),
    );
    xwl_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lc = label_col("satellite path", "path to xwayland-satellite binary  (empty = auto-detect)");
        lc.set_hexpand(true);
        row.append(&lc);
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_width_chars(20);
        entry.set_placeholder_text(Some("xwayland-satellite"));
        entry.set_text(&cfg.borrow().misc.xwayland_path);
        let cfg_s = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            cfg_s.borrow_mut().misc.xwayland_path = e.text().to_string();
        });
        row.append(&entry);
        xwl_card.append(&row);
    }
    page.append(&xwl_card);

    page.append(&save_button(cfg));
    page
}

fn idle_ms_label(ms: u32) -> String {
    if ms == 0 {
        "never".to_string()
    } else if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{:.1} s", ms as f64 / 1000.0)
    }
}

fn zoom_label(v: f64) -> String {
    if v <= 0.0 {
        "default".to_string()
    } else {
        format!("{:.0}%", v * 100.0)
    }
}

// ── Widget helpers ────────────────────────────────────────────────────────────

/// A scrollable content page with consistent padding.
fn page_box() -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 0);
    b.add_css_class("settings-content");
    b.set_margin_top(20);
    b.set_margin_start(20);
    b.set_margin_end(20);
    b.set_margin_bottom(20);
    b
}

/// Page title + subtitle header widget.
fn page_header(title: &str, sub: &str) -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 2);
    b.set_margin_bottom(6);
    let t = Label::new(Some(title));
    t.add_css_class("page-title");
    t.set_xalign(0.0);
    b.append(&t);
    if !sub.is_empty() {
        let s = Label::new(Some(sub));
        s.add_css_class("page-sub");
        s.set_xalign(0.0);
        b.append(&s);
    }
    b
}

/// A visually distinct card container with a title row.
fn card(title: &str) -> GtkBox {
    let outer = GtkBox::new(Orientation::Vertical, 0);
    outer.add_css_class("settings-card");

    let hdr = GtkBox::new(Orientation::Horizontal, 0);
    let lbl = Label::new(Some(&title.to_uppercase()));
    lbl.add_css_class("card-title");
    lbl.set_xalign(0.0);
    hdr.append(&lbl);
    outer.append(&hdr);

    // Separator below header
    let sep = Separator::new(Orientation::Horizontal);
    sep.add_css_class("row-sep");
    outer.append(&sep);

    outer
}

/// A horizontal row box used inside a card.
fn row_box() -> GtkBox {
    let r = GtkBox::new(Orientation::Horizontal, 12);
    r.add_css_class("settings-row");
    r
}

/// A vertical label + sublabel column.
fn label_col(label: &str, sub: &str) -> GtkBox {
    let col = GtkBox::new(Orientation::Vertical, 2);
    let lbl = Label::new(Some(label));
    lbl.add_css_class("row-label");
    lbl.set_xalign(0.0);
    col.append(&lbl);
    if !sub.is_empty() {
        let s = Label::new(Some(sub));
        s.add_css_class("row-sublabel");
        s.set_xalign(0.0);
        col.append(&s);
    }
    col
}

/// Appends a toggle (label + Switch) row to a card.
fn append_toggle<F: Fn(bool) + 'static>(
    card: &GtkBox,
    label: &str,
    sub: &str,
    initial: bool,
    on_change: F,
) {
    let row = row_box();
    let lc = label_col(label, sub);
    lc.set_hexpand(true);
    row.append(&lc);

    let sw = Switch::new();
    sw.set_active(initial);
    sw.set_valign(Align::Center);
    sw.connect_state_set(move |_, state| {
        on_change(state);
        glib::Propagation::Proceed
    });
    row.append(&sw);
    card.append(&row);
}

/// Appends a color hex-entry row to a card.
fn append_color<F: Fn(String) + 'static>(
    card: &GtkBox,
    label: &str,
    initial: &str,
    on_change: F,
) {
    let row = row_box();
    let lbl = Label::new(Some(label));
    lbl.add_css_class("row-label");
    lbl.set_xalign(0.0);
    lbl.set_hexpand(true);
    row.append(&lbl);

    let entry = Entry::new();
    entry.add_css_class("color-entry");
    entry.set_text(initial);
    entry.set_width_chars(10);
    entry.set_max_width_chars(10);
    entry.set_placeholder_text(Some("#rrggbb"));
    entry.connect_changed(move |e| {
        on_change(e.text().to_string());
    });
    row.append(&entry);
    card.append(&row);
}

/// Returns a "apply & save" button wired to `settings_backend::save`.
fn save_button(cfg: Rc<RefCell<SettingsConfig>>) -> Button {
    let btn = Button::with_label("apply & save");
    btn.add_css_class("save-btn");
    btn.set_halign(Align::End);
    btn.connect_clicked(move |b| {
        match crate::settings_backend::save(&cfg.borrow()) {
            Ok(()) => {
                b.set_label("saved ✓");
                let b2 = b.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                    b2.set_label("apply & save");
                });
            }
            Err(e) => {
                log::error!("settings: save failed: {e}");
                b.set_label("save failed ✗");
            }
        }
    });
    btn
}

// ── Closure helpers ───────────────────────────────────────────────────────────

/// Wraps a `Fn(&mut SettingsConfig, bool)` closure into a `Fn(bool)` that
/// borrows the shared config for mutation.
fn with_cfg<F>(cfg: Rc<RefCell<SettingsConfig>>, f: F) -> impl Fn(bool) + 'static
where
    F: Fn(&mut SettingsConfig, bool) + 'static,
{
    move |v| f(&mut cfg.borrow_mut(), v)
}

/// Same as `with_cfg` but for `String` values (color entries).
fn with_cfg_str<F>(cfg: Rc<RefCell<SettingsConfig>>, f: F) -> impl Fn(String) + 'static
where
    F: Fn(&mut SettingsConfig, String) + 'static,
{
    move |v| f(&mut cfg.borrow_mut(), v)
}

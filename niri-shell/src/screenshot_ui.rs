// Screenshot mode picker overlay.
// Toggled by SIGUSR2: `kill -USR2 $(pgrep niri-shell)` or a niri keybind:
//   bind "Super+Shift+S" { spawn ["sh" "-c" "kill -USR2 $(pgrep niri-shell)"]; }
// Dismiss with Escape or by clicking a mode button.

use std::sync::atomic::{AtomicBool, Ordering};

use gtk4::{
    glib,
    prelude::*,
    Application, ApplicationWindow, Box as GtkBox, Button, EventControllerKey, Label, Orientation,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};

static TOGGLE_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn sigusr2_handler(_: libc::c_int) {
    TOGGLE_REQUESTED.store(true, Ordering::Relaxed);
}

const CSS: &str = r#"
.ss-root {
    background: rgba(13, 13, 23, 0.92);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 14px;
    padding: 18px;
    font-family: "Inter", "Noto Sans", sans-serif;
}

.ss-title {
    font-size: 11px;
    color: #565f89;
    letter-spacing: 0.08em;
    margin-bottom: 12px;
}

.ss-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 10px;
    padding: 14px 20px;
    color: #c0caf5;
    font-family: "Inter", "Noto Sans", sans-serif;
    font-size: 12px;
    min-width: 84px;
}

.ss-btn:hover {
    background: rgba(122, 162, 247, 0.16);
    border-color: rgba(122, 162, 247, 0.35);
    color: #7aa2f7;
}

.ss-icon {
    font-size: 20px;
    margin-bottom: 6px;
}
"#;

pub fn build_screenshot_window(app: &Application) {
    // Install SIGUSR2 handler.
    // SAFETY: sigusr2_handler only writes an AtomicBool — async-signal-safe.
    unsafe {
        libc::signal(libc::SIGUSR2, sigusr2_handler as *const () as libc::sighandler_t);
    }

    if let Some(display) = gtk4::gdk::Display::default() {
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(CSS);
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let window = ApplicationWindow::builder()
        .application(app)
        .title("niri-screenshot")
        .decorated(false)
        .resizable(false)
        .build();

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.add_css_class("ss-root");

    let title = Label::new(Some("SCREENSHOT"));
    title.add_css_class("ss-title");
    title.set_halign(gtk4::Align::Center);
    root.append(&title);

    let btn_row = GtkBox::new(Orientation::Horizontal, 10);
    btn_row.set_halign(gtk4::Align::Center);

    let region_btn = make_mode_btn("✂", "Region");
    let window_btn = make_mode_btn("⬜", "Window");
    let full_btn = make_mode_btn("🖥", "Full");

    btn_row.append(&region_btn);
    btn_row.append(&window_btn);
    btn_row.append(&full_btn);
    root.append(&btn_row);

    window.set_child(Some(&root));
    window.set_visible(false);

    // ── Button handlers ──────────────────────────────────────────────────────

    let win_r = window.downgrade();
    region_btn.connect_clicked(move |_| {
        spawn_screenshot("region");
        if let Some(w) = win_r.upgrade() {
            w.set_visible(false);
        }
    });

    let win_w = window.downgrade();
    window_btn.connect_clicked(move |_| {
        spawn_screenshot("window");
        if let Some(w) = win_w.upgrade() {
            w.set_visible(false);
        }
    });

    let win_f = window.downgrade();
    full_btn.connect_clicked(move |_| {
        spawn_screenshot("full");
        if let Some(w) = win_f.upgrade() {
            w.set_visible(false);
        }
    });

    // ── Escape to dismiss ────────────────────────────────────────────────────

    {
        let win_weak = window.downgrade();
        let key_ctrl = EventControllerKey::new();
        key_ctrl.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gtk4::gdk::Key::Escape {
                if let Some(w) = win_weak.upgrade() {
                    w.set_visible(false);
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        window.add_controller(key_ctrl);
    }

    // ── SIGUSR2 poll — toggle visibility ─────────────────────────────────────

    let win_weak = window.downgrade();
    glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
        if !TOGGLE_REQUESTED.swap(false, Ordering::Relaxed) {
            return glib::ControlFlow::Continue;
        }
        let Some(w) = win_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if w.is_visible() {
            w.set_visible(false);
        } else {
            w.present();
        }
        glib::ControlFlow::Continue
    });
}

fn make_mode_btn(icon: &str, label: &str) -> Button {
    let btn = Button::new();
    btn.add_css_class("ss-btn");
    let col = GtkBox::new(Orientation::Vertical, 4);
    col.set_halign(gtk4::Align::Center);
    let icon_lbl = Label::new(Some(icon));
    icon_lbl.add_css_class("ss-icon");
    let name_lbl = Label::new(Some(label));
    col.append(&icon_lbl);
    col.append(&name_lbl);
    btn.set_child(Some(&col));
    btn
}

fn spawn_screenshot(mode: &str) {
    let cmd: String = match mode {
        "region" => "grim -g \"$(slurp)\" - | wl-copy".to_owned(),
        "window" => "grim -g \"$(slurp -w)\" - | wl-copy".to_owned(),
        _ => "grim - | wl-copy".to_owned(),
    };
    std::thread::spawn(move || {
        if let Err(e) = std::process::Command::new("sh").args(["-c", &cmd]).spawn() {
            log::error!("screenshot command failed: {e}");
        }
    });
}

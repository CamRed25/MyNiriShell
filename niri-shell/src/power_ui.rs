// GTK4 power action dialog — zero business logic.
// Shows Suspend / Reboot / Shut Down in a small centred modal window.

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, Box as GtkBox, Button, Label, Orientation, Window};

const CSS: &str = "
.power-dialog {
    background: rgba(15, 15, 25, 0.97);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 14px;
    padding: 20px 20px 16px;
}
.power-title {
    font-size: 15px;
    font-weight: 600;
    color: #c0caf5;
    margin-bottom: 4px;
}
.power-btn {
    background: rgba(122, 162, 247, 0.14);
    border: 1px solid rgba(122, 162, 247, 0.25);
    border-radius: 10px;
    color: #c0caf5;
    font-size: 12px;
    padding: 10px 14px;
    min-width: 72px;
}
.power-btn:hover {
    background: rgba(122, 162, 247, 0.28);
}
.power-cancel {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 8px;
    color: #565f89;
    font-size: 11px;
    padding: 4px 20px;
    margin-top: 4px;
}
.power-cancel:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #a9b1d6;
}
";

/// Show the power action modal. Must be called from the GTK main thread.
pub fn show_power_dialog(app: &Application) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let dialog = Window::builder()
        .application(app)
        .title("Power")
        .decorated(false)
        .resizable(false)
        .default_width(310)
        .modal(true)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 14);
    root.add_css_class("power-dialog");

    let title = Label::new(Some("Power"));
    title.add_css_class("power-title");
    title.set_halign(gtk4::Align::Start);
    root.append(&title);

    let btn_row = GtkBox::new(Orientation::Horizontal, 8);
    btn_row.set_halign(gtk4::Align::Center);

    let suspend_btn = make_power_btn("⏸ Suspend");
    let reboot_btn = make_power_btn("↺ Reboot");
    let shutdown_btn = make_power_btn("⏻ Shut Down");

    {
        let dlg = dialog.downgrade();
        suspend_btn.connect_clicked(move |_| {
            if let Some(d) = dlg.upgrade() {
                d.close();
            }
            std::thread::spawn(|| {
                if let Err(e) = crate::power_backend::suspend() {
                    log::error!("suspend: {e}");
                }
            });
        });
    }
    {
        let dlg = dialog.downgrade();
        reboot_btn.connect_clicked(move |_| {
            if let Some(d) = dlg.upgrade() {
                d.close();
            }
            std::thread::spawn(|| {
                if let Err(e) = crate::power_backend::reboot() {
                    log::error!("reboot: {e}");
                }
            });
        });
    }
    {
        let dlg = dialog.downgrade();
        shutdown_btn.connect_clicked(move |_| {
            if let Some(d) = dlg.upgrade() {
                d.close();
            }
            std::thread::spawn(|| {
                if let Err(e) = crate::power_backend::poweroff() {
                    log::error!("poweroff: {e}");
                }
            });
        });
    }

    btn_row.append(&suspend_btn);
    btn_row.append(&reboot_btn);
    btn_row.append(&shutdown_btn);
    root.append(&btn_row);

    let cancel_btn = Button::with_label("Cancel");
    cancel_btn.add_css_class("power-cancel");
    cancel_btn.set_halign(gtk4::Align::Center);
    {
        let dlg = dialog.downgrade();
        cancel_btn.connect_clicked(move |_| {
            if let Some(d) = dlg.upgrade() {
                d.close();
            }
        });
    }
    root.append(&cancel_btn);

    // Escape closes the dialog.
    let key_ctrl = gtk4::EventControllerKey::new();
    {
        let dlg = dialog.downgrade();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                if let Some(d) = dlg.upgrade() {
                    d.close();
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    dialog.add_controller(key_ctrl);

    dialog.set_child(Some(&root));
    dialog.present();
}

fn make_power_btn(label: &str) -> Button {
    let btn = Button::with_label(label);
    btn.add_css_class("power-btn");
    btn
}

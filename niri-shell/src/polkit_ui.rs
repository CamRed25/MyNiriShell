//! GTK4 modal dialog for Polkit authentication.
// Receives a PolkitRequest (with a response channel) and prompts for password.

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{Orientation, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use std::rc::Rc;
use std::cell::RefCell;
use crate::polkit_agent::PolkitRequest;

pub struct PolkitDialog {
    window: gtk::Window,
}

impl PolkitDialog {
    /// Build and wire the dialog. Takes ownership of `req` for the response channel.
    pub fn new(parent: &gtk::Window, req: PolkitRequest) -> Rc<RefCell<Self>> {
        let window = gtk::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Authentication Required")
            .default_width(340)
            .resizable(false)
            .build();
        window.set_decorated(false);

        let css = CssProvider::new();
        css.load_from_string(r#"
            window {
                background: rgba(15,15,25,0.82);
                border-radius: 14px;
                border: 1px solid rgba(255,255,255,0.08);
            }
            .polkit-body { padding: 20px 24px 0 24px; }
            .polkit-app-icon { margin-bottom: 8px; }
            .polkit-app-name { color: #c0caf5; font-size: 15px; font-weight: 500; }
            .polkit-action { color: #565f89; font-size: 12px; }
            .polkit-details-btn {
                background: none; border: none; box-shadow: none;
                color: #3b3f5c; font-size: 10px; padding: 6px 0 2px 0;
            }
            .polkit-details-pill {
                background: rgba(255,255,255,0.03);
                border-radius: 6px;
                border: 1px solid rgba(255,255,255,0.06);
                color: #3b3f5c; font-size: 10px;
                padding: 7px 10px; margin-top: 4px;
            }
            .polkit-pw-label { color: #565f89; font-size: 11px; margin-top: 12px; }
            entry.polkit-pw-entry {
                background: rgba(255,255,255,0.04);
                border-radius: 8px;
                border: 1px solid rgba(255,255,255,0.1);
                color: #c0caf5; caret-color: #7aa2f7;
                margin-top: 4px;
            }
            .polkit-spinner-row { color: #565f89; font-size: 11px; margin-top: 6px; }
            .polkit-footer { padding: 16px 24px 20px 24px; }
            .polkit-btn {
                border-radius: 8px; font-size: 12px; font-weight: 500;
                padding: 6px 14px;
            }
            .polkit-btn.cancel {
                background: rgba(255,255,255,0.04);
                border: 1px solid rgba(255,255,255,0.08);
                color: #565f89;
            }
            .polkit-btn.primary {
                background: rgba(122,162,247,0.15);
                border: 1px solid rgba(122,162,247,0.3);
                color: #7aa2f7;
            }
        "#);
        gtk::style_context_add_provider_for_display(
            &gtk::prelude::WidgetExt::display(&window),
            &css,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // ── outer vertial container ──────────────────────────────────────────
        let outer = gtk::Box::new(Orientation::Vertical, 0);

        // ── header / body ────────────────────────────────────────────────────
        let body = gtk::Box::new(Orientation::Vertical, 0);
        body.add_css_class("polkit-body");

        let app_icon = gtk::Image::from_icon_name("dialog-password");
        app_icon.set_pixel_size(48);
        app_icon.set_halign(gtk::Align::Center);
        app_icon.add_css_class("polkit-app-icon");
        body.append(&app_icon);

        let app_name = gtk::Label::new(Some("Authentication Required"));
        app_name.set_halign(gtk::Align::Center);
        app_name.add_css_class("polkit-app-name");
        body.append(&app_name);

        let action_lbl = gtk::Label::new(Some(&req.message));
        action_lbl.set_halign(gtk::Align::Center);
        action_lbl.set_wrap(true);
        action_lbl.add_css_class("polkit-action");
        body.append(&action_lbl);

        // ── details row ──────────────────────────────────────────────────────
        let details_btn = gtk::Button::with_label("▶ show details");
        details_btn.add_css_class("polkit-details-btn");
        details_btn.set_halign(gtk::Align::Start);
        body.append(&details_btn);

        let details_text: String = req
            .details
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");
        let details_pill = gtk::Label::new(Some(&details_text));
        details_pill.add_css_class("polkit-details-pill");
        details_pill.set_xalign(0.0);
        details_pill.set_visible(false);
        body.append(&details_pill);

        {
            let details_pill = details_pill.clone();
            let expanded = Rc::new(RefCell::new(false));
            details_btn.connect_clicked(move |btn| {
                let mut e = expanded.borrow_mut();
                *e = !*e;
                details_pill.set_visible(*e);
                btn.set_label(if *e { "▼ hide details" } else { "▶ show details" });
            });
        }

        // ── password field ───────────────────────────────────────────────────
        let pw_label = gtk::Label::new(Some(&format!(
            "Password for {}",
            std::env::var("USER").unwrap_or_else(|_| "user".into())
        )));
        pw_label.set_halign(gtk::Align::Start);
        pw_label.add_css_class("polkit-pw-label");
        body.append(&pw_label);

        let pw_entry = gtk::Entry::new();
        pw_entry.set_visibility(false);
        pw_entry.set_placeholder_text(Some("Enter password…"));
        pw_entry.add_css_class("polkit-pw-entry");
        body.append(&pw_entry);

        // ── spinner row (shown during authentication) ─────────────────────────
        let spinner_row = gtk::Box::new(Orientation::Horizontal, 6);
        spinner_row.set_halign(gtk::Align::Center);
        spinner_row.add_css_class("polkit-spinner-row");
        let spinner = gtk::Spinner::new();
        spinner_row.append(&spinner);
        spinner_row.append(&gtk::Label::new(Some("Verifying…")));
        spinner_row.set_visible(false);
        body.append(&spinner_row);

        outer.append(&body);

        // ── footer ───────────────────────────────────────────────────────────
        let footer = gtk::Box::new(Orientation::Horizontal, 8);
        footer.add_css_class("polkit-footer");
        footer.set_halign(gtk::Align::End);

        let cancel_btn = gtk::Button::with_label("Cancel");
        cancel_btn.add_css_class("polkit-btn");
        cancel_btn.add_css_class("cancel");

        let auth_btn = gtk::Button::with_label("Authenticate");
        auth_btn.add_css_class("polkit-btn");
        auth_btn.add_css_class("primary");

        footer.append(&cancel_btn);
        footer.append(&auth_btn);
        outer.append(&footer);

        window.set_child(Some(&outer));

        // ── response channel shared between buttons ───────────────────────────
        let response_tx = Rc::new(RefCell::new(Some(req.response_tx)));

        // Helper closure: trigger authentication
        let do_auth = {
            let pw_entry = pw_entry.clone();
            let response_tx = response_tx.clone();
            let auth_btn = auth_btn.clone();
            let cancel_btn = cancel_btn.clone();
            let spinner_row = spinner_row.clone();
            let spinner = spinner.clone();
            move || {
                let pw = pw_entry.text().to_string();
                if let Some(tx) = response_tx.borrow_mut().take() {
                    let _ = tx.send(Some(pw));
                }
                spinner_row.set_visible(true);
                spinner.start();
                auth_btn.set_sensitive(false);
                cancel_btn.set_sensitive(false);
            }
        };

        // Authenticate button
        auth_btn.connect_clicked({
            let do_auth = do_auth.clone();
            move |_| do_auth()
        });

        // Enter key in password field
        pw_entry.connect_activate(move |_| do_auth());

        // Cancel button
        cancel_btn.connect_clicked({
            let window = window.clone();
            move |_| {
                if let Some(tx) = response_tx.borrow_mut().take() {
                    let _ = tx.send(None);
                }
                window.close();
            }
        });

        // Close window when authentication response fires (agent closes dialog)
        // (window is destroyed when dropped; nothing extra needed here)

        Rc::new(RefCell::new(Self { window }))
    }

    pub fn show(&self) {
        self.window.set_visible(true);
    }
}


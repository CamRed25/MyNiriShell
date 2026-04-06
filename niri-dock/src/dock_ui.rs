// dock_ui.rs — GTK4 dock user interface.
// UI-only: no business logic — all state and logic lives in dock_backend.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, Image, Label, Orientation,
    Overlay, Separator,
};
use thiserror::Error;

use crate::dock_backend::{DockItem, DockState};

const APP_ID: &str = "com.niri.dock";

const CSS: &str = "
window {
    background: transparent;
}

.dock-bar {
    background: rgba(13, 13, 23, 0.82);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 14px;
    padding: 6px 12px;
}

.dock-icon-btn {
    background: rgba(122, 162, 247, 0.12);
    border-radius: 9px;
    border: none;
    padding: 6px;
    min-width: 38px;
    min-height: 38px;
    margin-top: 4px;
    margin-bottom: 0;
    transition: margin 150ms ease, background 150ms ease;
}

.dock-icon-btn:hover {
    margin-top: 0;
    margin-bottom: 4px;
    background: rgba(122, 162, 247, 0.22);
}

.dock-dot {
    background: #7aa2f7;
    border-radius: 50%;
    min-width: 4px;
    min-height: 4px;
    max-width: 4px;
    max-height: 4px;
}

.dock-dot-empty {
    background: transparent;
    border-radius: 50%;
    min-width: 4px;
    min-height: 4px;
    max-width: 4px;
    max-height: 4px;
}

.dock-badge {
    background: #f7768e;
    border-radius: 50%;
    min-width: 14px;
    min-height: 14px;
    color: white;
    font-size: 8px;
    font-weight: 700;
}

.dock-separator {
    background: rgba(255, 255, 255, 0.1);
    min-width: 1px;
    min-height: 32px;
    margin: 0 4px;
}
";

#[derive(Debug, Error)]
pub enum DockUiError {
    #[error("GTK failed to initialize")]
    GtkInit,
}

pub fn run() -> Result<(), DockUiError> {
    gtk4::init().map_err(|_| DockUiError::GtkInit)?;

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let state = Rc::new(RefCell::new(DockState::new()));

        // Populate active windows; in production these come via compositor IPC.
        state.borrow_mut().set_active_windows(vec![
            DockItem::active("firefox", "Firefox", "firefox"),
            DockItem::active("org.gnome.Terminal", "Terminal", "org.gnome.Terminal"),
            DockItem::active("code", "VS Code", "com.visualstudio.code"),
        ]);

        load_css();
        build_window(app, state);
    });

    app.run();
    Ok(())
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_window(app: &Application, state: Rc<RefCell<DockState>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("niri-dock")
        .decorated(false)
        .resizable(false)
        .build();

    let dock = build_dock(state);
    window.set_child(Some(&dock));
    window.present();
}

fn build_dock(state: Rc<RefCell<DockState>>) -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.add_css_class("dock-bar");
    bar.set_valign(gtk4::Align::Center);

    // Left section: active workspace apps in open order.
    let active_section = GtkBox::new(Orientation::Horizontal, 4);
    {
        let s = state.borrow();
        for item in &s.active {
            active_section.append(&build_dock_item(item, None, state.clone()));
        }
    }
    bar.append(&active_section);

    // Separator between sections.
    let sep = Separator::new(Orientation::Vertical);
    sep.add_css_class("dock-separator");
    bar.append(&sep);

    // Right section: pinned apps with drag-and-drop reordering.
    let pinned_section = GtkBox::new(Orientation::Horizontal, 4);
    populate_pinned_section(&pinned_section, state);
    bar.append(&pinned_section);

    bar
}

/// Clears and repopulates the pinned section.  Called on initial build and after D&D reorder.
fn populate_pinned_section(section: &GtkBox, state: Rc<RefCell<DockState>>) {
    while let Some(child) = section.first_child() {
        section.remove(&child);
    }

    let items: Vec<DockItem> = state.borrow().pinned.clone();

    for (idx, item) in items.iter().enumerate() {
        // Reflect running state: highlight dot when pinned app is currently active.
        let running = state.borrow().active.iter().any(|a| a.id == item.id);
        let mut display_item = item.clone();
        display_item.is_active = running;

        let item_box = build_dock_item(&display_item, None, state.clone());

        // Drag source: broadcast own index when a drag starts.
        let drag_source = gtk4::DragSource::new();
        drag_source.set_actions(gdk::DragAction::MOVE);
        let idx_str = idx.to_string();
        drag_source.connect_prepare(move |_src, _x, _y| {
            Some(gdk::ContentProvider::for_value(&idx_str.to_value()))
        });
        item_box.add_controller(drag_source);

        // Drop target: receive dragged index and reorder.
        let drop_target = gtk4::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
        let state_drop = state.clone();
        let section_drop = section.clone();
        drop_target.connect_drop(move |_target, value, _x, _y| {
            let Ok(from_str) = value.get::<String>() else {
                return false;
            };
            let Ok(from) = from_str.parse::<usize>() else {
                return false;
            };
            {
                let mut s = state_drop.borrow_mut();
                if let Err(e) = s.reorder_pinned(from, idx) {
                    log::warn!("reorder_pinned({from} → {idx}): {e}");
                    return false;
                }
            }
            populate_pinned_section(&section_drop, state_drop.clone());
            true
        });
        item_box.add_controller(drop_target);

        section.append(&item_box);
    }
}

/// Builds a single dock item: [overlay(icon button + optional badge), indicator dot].
fn build_dock_item(
    item: &DockItem,
    badge_count: Option<u32>,
    state: Rc<RefCell<DockState>>,
) -> GtkBox {
    let item_box = GtkBox::new(Orientation::Vertical, 3);
    item_box.set_halign(gtk4::Align::Center);

    let overlay = Overlay::new();

    let btn = Button::new();
    btn.add_css_class("dock-icon-btn");
    let icon = Image::from_icon_name(&item.icon);
    icon.set_pixel_size(22);
    btn.set_child(Some(&icon));
    btn.set_tooltip_text(Some(&item.name));
    overlay.set_child(Some(&btn));

    if let Some(count) = badge_count {
        if count > 0 {
            let badge = Label::new(Some(&count.to_string()));
            badge.add_css_class("dock-badge");
            badge.set_halign(gtk4::Align::End);
            badge.set_valign(gtk4::Align::Start);
            overlay.add_overlay(&badge);
        }
    }

    let item_for_click = item.clone();
    let state_for_click = state;
    btn.connect_clicked(move |_| {
        let s = state_for_click.borrow();
        if let Err(e) = s.launch(&item_for_click) {
            log::error!("launch '{}': {e}", item_for_click.name);
        }
    });

    item_box.append(&overlay);

    // Indicator dot: filled (#7aa2f7) when active, transparent when idle.
    let dot = GtkBox::new(Orientation::Horizontal, 0);
    dot.set_halign(gtk4::Align::Center);
    dot.set_size_request(4, 4);
    if item.is_active {
        dot.add_css_class("dock-dot");
    } else {
        dot.add_css_class("dock-dot-empty");
    }
    item_box.append(&dot);

    item_box
}

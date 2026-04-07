// GTK4 dock user interface.
// UI-only: no business logic — all state and logic lives in dock_backend.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, Image, Label, Orientation, Overlay,
    Separator,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::dock_backend::{DockItem, DockState};

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
}

.dock-dot-empty {
    background: transparent;
    border-radius: 50%;
    min-width: 4px;
    min-height: 4px;
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

.dock-ghost {
    opacity: 0.45;
    filter: grayscale(0.6);
}
";

/// Called once inside `app.connect_activate`. Loads CSS and builds the dock window.
pub fn build_dock_window(app: &Application, state: Rc<RefCell<DockState>>) {
    load_css();
    build_window(app, state);
}

fn load_css() {
    let Some(display) = gdk::Display::default() else {
        log::warn!("dock_ui: no GDK display, skipping CSS load");
        return;
    };
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(CSS);
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_window(app: &Application, state: Rc<RefCell<DockState>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("niri-dock")
        .decorated(false)
        .resizable(false)
        .build();

    // Pin to the bottom edge, centred, above all windows.
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.auto_exclusive_zone_enable();
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);
    window.set_anchor(Edge::Top, false);

    let dock = build_dock(state);
    window.set_child(Some(&dock));
    window.present();
}

fn build_dock(state: Rc<RefCell<DockState>>) -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.add_css_class("dock-bar");
    bar.set_valign(gtk4::Align::Center);

    // Left section: active workspace apps — rebuilt every 500 ms from state.
    let active_section = GtkBox::new(Orientation::Horizontal, 4);
    refresh_active_section(&active_section, &state);
    bar.append(&active_section);

    // Separator between sections — hidden when no windows are open.
    let sep = Separator::new(Orientation::Vertical);
    sep.add_css_class("dock-separator");
    sep.set_visible(!state.borrow().active.is_empty());
    bar.append(&sep);

    // Right section: pinned apps with drag-and-drop reordering.
    let pinned_section = GtkBox::new(Orientation::Horizontal, 4);
    populate_pinned_section(&pinned_section, Rc::clone(&state));
    bar.append(&pinned_section);

    // Poll every 500 ms to reflect open-window and focus changes.
    let state_for_ghost = Rc::clone(&state);
    let active_weak = active_section.downgrade();
    let pinned_weak = pinned_section.downgrade();
    let sep_weak = sep.downgrade();
    glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        let (Some(active), Some(pinned)) = (active_weak.upgrade(), pinned_weak.upgrade()) else {
            return glib::ControlFlow::Break;
        };
        // Prune expired ghosts before re-rendering.
        state.borrow_mut().prune_ghosts();
        refresh_active_section(&active, &state);
        refresh_pinned_section(&pinned, &state);
        if let Some(s) = sep_weak.upgrade() {
            let dock = state.borrow();
            s.set_visible(!dock.active.is_empty() || !dock.ghosts.is_empty());
        }
        glib::ControlFlow::Continue
    });

    // Fast ghost-fade timer (80 ms) — only redraws active section when ghosts exist.
    let active_fade = active_section.downgrade();
    let state_fade = Rc::clone(&state_for_ghost);
    glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
        let Some(active) = active_fade.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let has_ghosts = !state_fade.borrow().ghosts.is_empty();
        if has_ghosts {
            state_fade.borrow_mut().prune_ghosts();
            refresh_active_section(&active, &state_fade);
        }
        glib::ControlFlow::Continue
    });

    bar
}

/// Encodes the full visible state of the active section as a string key.
fn active_section_key(items: &[crate::dock_backend::DockItem], ghosts: &[crate::dock_backend::GhostItem]) -> String {
    let mut key = items
        .iter()
        .map(|i| format!("{}:{}:{}", i.id, i.is_active as u8, i.workspace_id))
        .collect::<Vec<_>>()
        .join("|");
    if !ghosts.is_empty() {
        key.push_str("~ghosts:");
        for g in ghosts {
            // Include approximate opacity bucket (0..10) so we redraw during fade.
            let bucket = (g.opacity() * 10.0) as u8;
            key.push_str(&format!("{}:{bucket}", g.id));
        }
    }
    key
}

/// Encodes pinned-section running/focused state as a string key.
fn pinned_section_key(state: &Rc<RefCell<DockState>>) -> String {
    let dock = state.borrow();
    let focused = dock.active.iter().find(|a| a.is_active).map(|a| a.id.clone());
    let active_ids: std::collections::HashSet<&str> =
        dock.active.iter().map(|a| a.id.as_str()).collect();
    let mut key = dock.pinned
        .iter()
        .map(|p| {
            let running = active_ids.contains(p.id.as_str());
            let focused = focused.as_deref() == Some(p.id.as_str());
            let badge = dock.notif_counts.get(&p.id).copied().unwrap_or(0);
            format!("{}:{}:{}:{badge}", p.id, running as u8, focused as u8)
        })
        .collect::<Vec<_>>()
        .join("|");
    // Append a summary of active-section badge counts so pinned re-renders on change.
    if !dock.notif_counts.is_empty() {
        key.push_str("~nc:");
        let mut nc: Vec<_> = dock.notif_counts.iter().collect();
        nc.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in nc {
            key.push_str(&format!("{k}:{v}|"));
        }
    }
    key
}

fn refresh_active_section(section: &GtkBox, state: &Rc<RefCell<DockState>>) {
    let dock = state.borrow();
    let items = dock.active.clone();
    let ghosts = dock.ghosts.clone();
    let notif_counts = dock.notif_counts.clone();
    drop(dock);
    let key = active_section_key(&items, &ghosts);
    if section.widget_name().as_str() == key {
        return;
    }
    while let Some(child) = section.first_child() {
        section.remove(&child);
    }

    // Group by workspace_id preserving insertion order.
    let mut seen: Vec<u64> = Vec::new();
    let mut groups: Vec<(u64, Vec<DockItem>)> = Vec::new();
    for item in items {
        let wid = item.workspace_id;
        if let Some(pos) = seen.iter().position(|&w| w == wid) {
            groups[pos].1.push(item);
        } else {
            seen.push(wid);
            groups.push((wid, vec![item]));
        }
    }

    let show_labels = groups.len() > 1;
    for (ws_id, group_items) in &groups {
        if show_labels {
            let col = GtkBox::new(Orientation::Vertical, 1);
            let lbl = Label::new(Some(&format!("ws {ws_id}")));
            lbl.add_css_class("dock-ws-label");
            lbl.set_halign(gtk4::Align::Center);
            col.append(&lbl);
            let row_box = GtkBox::new(Orientation::Horizontal, 4);
            for item in group_items {
                let badge = notif_counts.get(&item.id).copied();
                row_box.append(&build_dock_item(item, badge, Rc::clone(state)));
            }
            col.append(&row_box);
            section.append(&col);
        } else {
            for item in group_items {
                let badge = notif_counts.get(&item.id).copied();
                section.append(&build_dock_item(item, badge, Rc::clone(state)));
            }
        }
    }

    // Render ghost icons after active windows.
    for ghost in &ghosts {
        let ghost_box = build_ghost_item(ghost);
        section.append(&ghost_box);
    }

    section.set_widget_name(&key);
}

fn refresh_pinned_section(section: &GtkBox, state: &Rc<RefCell<DockState>>) {
    let key = pinned_section_key(state);
    if section.widget_name().as_str() == key {
        return;
    }
    populate_pinned_section(section, Rc::clone(state));
    section.set_widget_name(&key);
}

/// Resolve an icon name against the current GTK icon theme, falling back to a generic icon.
fn resolve_icon(name: &str) -> String {
    if name.is_empty() {
        return "application-x-executable".to_owned();
    }
    if let Some(display) = gdk::Display::default() {
        let theme = gtk4::IconTheme::for_display(&display);
        if theme.has_icon(name) {
            return name.to_owned();
        }
        let lower = name.to_lowercase();
        if theme.has_icon(&lower) {
            return lower;
        }
        let sym = format!("{lower}-symbolic");
        if theme.has_icon(&sym) {
            return sym;
        }
    }
    "application-x-executable".to_owned()
}

fn populate_pinned_section(section: &GtkBox, state: Rc<RefCell<DockState>>) {
    while let Some(child) = section.first_child() {
        section.remove(&child);
    }

    let items: Vec<DockItem> = state.borrow().pinned.clone();

    for (idx, item) in items.iter().enumerate() {
        let (running, badge) = {
            let dock = state.borrow();
            let r = dock.active.iter().any(|a| a.id == item.id);
            let b = dock.notif_counts.get(&item.id).copied();
            (r, b)
        };
        let mut display_item = item.clone();
        display_item.is_active = running;

        let item_box = build_dock_item(&display_item, badge, state.clone());

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

fn build_ghost_item(ghost: &crate::dock_backend::GhostItem) -> GtkBox {
    let item_box = GtkBox::new(Orientation::Vertical, 3);
    item_box.set_halign(gtk4::Align::Center);
    item_box.set_opacity(ghost.opacity());
    item_box.add_css_class("dock-ghost");

    let btn = gtk4::Button::new();
    btn.add_css_class("dock-icon-btn");
    let icon = Image::from_icon_name(&resolve_icon(&ghost.icon));
    icon.set_pixel_size(22);
    btn.set_child(Some(&icon));
    btn.set_tooltip_text(Some(&ghost.name));
    btn.set_sensitive(false);
    item_box.append(&btn);

    // Empty dot placeholder to keep vertical alignment consistent.
    let dot = GtkBox::new(Orientation::Horizontal, 0);
    dot.set_halign(gtk4::Align::Center);
    dot.set_size_request(4, 4);
    dot.add_css_class("dock-dot-empty");
    item_box.append(&dot);

    item_box
}

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
    let icon = Image::from_icon_name(&resolve_icon(&item.icon));
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
        if item_for_click.niri_id != 0 {
            // Window already open — focus it via niri IPC.
            if let Err(e) = crate::ipc::send_action(crate::ipc::types::NiriAction::FocusWindow {
                id: item_for_click.niri_id,
            }) {
                log::warn!("focus window {}: {e}", item_for_click.niri_id);
            }
        } else {
            let s = state_for_click.borrow();
            if let Err(e) = s.launch(&item_for_click) {
                log::error!("launch '{}': {e}", item_for_click.name);
            }
        }
    });

    item_box.append(&overlay);

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

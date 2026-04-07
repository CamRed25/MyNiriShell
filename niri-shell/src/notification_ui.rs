// GTK4 UI for notifications — zero business logic.
//
// Exports:
//   • `NotificationToasts`  — layer-shell overlay that stacks live toast cards top-right.
//   • `build_notifications_popover` — bell-button popover with a live-refresh list.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk4::{
    glib,
    prelude::*,
    Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation, Popover, ScrolledWindow,
    Window,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::panel_backend::PanelState;

// ── CSS ───────────────────────────────────────────────────────────────────────

const TOAST_CSS: &str = r#"
.toast {
    background: rgba(15, 15, 25, 0.92);
    border: 1px solid rgba(122, 162, 247, 0.25);
    border-radius: 12px;
    padding: 10px 14px;
    min-width: 280px;
}
.toast-app     { font-size: 10px; color: #565f89; font-family: "Inter", sans-serif; }
.toast-summary { font-size: 13px; font-weight: bold; color: #c0caf5;
                 font-family: "Inter", sans-serif; }
.toast-body    { font-size: 11px; color: #a9b1d6; font-family: "Inter", sans-serif; }
.toast-close   { background: transparent; border: none; color: #565f89;
                 padding: 0 4px; min-height: 0; min-width: 0; }
.toast-close:hover { color: #c0caf5; }
"#;

const NOTIF_CSS: &str = r#"
.nc-title      { font-size: 12px; font-weight: bold; color: #c0caf5; margin-bottom: 6px; }
.notif-row     { padding: 6px 4px; }
.notif-app     { font-size: 10px; color: #565f89; }
.notif-summary { font-size: 12px; color: #c0caf5; font-weight: bold; }
.notif-body    { font-size: 11px; color: #a9b1d6; }
.notif-dismiss { background: transparent; border: none; color: #565f89;
                 padding: 0 4px; min-height: 0; min-width: 0; }
.notif-dismiss:hover { color: #c0caf5; }
.notif-empty   { font-size: 12px; color: #565f89; padding: 8px 4px; }
"#;

fn load_css(data: &str) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(data);
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

// ── NotificationToasts ────────────────────────────────────────────────────────

/// Stacks live toast notifications in a layer-shell overlay anchored top-right.
pub struct NotificationToasts {
    window: Window,
    list: GtkBox,
    count: Rc<Cell<u32>>,
}

impl NotificationToasts {
    pub fn new(app: &gtk4::Application) -> Rc<Self> {
        load_css(TOAST_CSS);

        let window = Window::builder()
            .application(app)
            .decorated(false)
            .resizable(false)
            .build();
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Bottom, false);
        window.set_margin(Edge::Top, 52); // sit below the panel bar
        window.set_margin(Edge::Right, 12);
        window.set_exclusive_zone(0);

        let list = GtkBox::new(Orientation::Vertical, 6);
        list.set_margin_top(4);
        list.set_margin_bottom(4);
        list.set_margin_start(4);
        list.set_margin_end(4);
        window.set_child(Some(&list));

        Rc::new(Self { window, list, count: Rc::new(Cell::new(0)) })
    }

    /// Display a toast card. The caller should suppress the call when DND is active.
    pub fn show_toast(self: &Rc<Self>, app: &str, summary: &str, body: &str, timeout_ms: i32) {
        let toast = build_toast_widget(app, summary, body);
        self.list.append(&toast);

        let n = self.count.get() + 1;
        self.count.set(n);
        if n == 1 {
            self.window.present();
        }

        // Select auto-dismiss delay: -1 or 0 → 5 s default.
        let dismiss_delay = if timeout_ms > 0 { timeout_ms as u64 } else { 5_000 };

        // Auto-dismiss timer.
        {
            let win = self.window.clone();
            let list = self.list.clone();
            let count = Rc::clone(&self.count);
            let t = toast.clone();
            glib::timeout_add_local_once(Duration::from_millis(dismiss_delay), move || {
                remove_toast(&win, &list, &t, &count);
            });
        }

        // Manual close button — it is the last child of the header (first child of toast).
        if let Some(header) = toast.first_child() {
            if let Some(close) = header.last_child() {
                if let Ok(btn) = close.downcast::<Button>() {
                    let win = self.window.clone();
                    let list = self.list.clone();
                    let count = Rc::clone(&self.count);
                    let t = toast.clone();
                    btn.connect_clicked(move |_| remove_toast(&win, &list, &t, &count));
                }
            }
        }
    }
}

fn remove_toast(win: &Window, list: &GtkBox, toast: &GtkBox, count: &Rc<Cell<u32>>) {
    list.remove(toast);
    let remaining = count.get().saturating_sub(1);
    count.set(remaining);
    if remaining == 0 {
        win.set_visible(false);
    }
}

fn build_toast_widget(app: &str, summary: &str, body: &str) -> GtkBox {
    let toast = GtkBox::new(Orientation::Vertical, 4);
    toast.add_css_class("toast");

    let header = GtkBox::new(Orientation::Horizontal, 0);
    let app_lbl = Label::new(Some(app));
    app_lbl.add_css_class("toast-app");
    app_lbl.set_hexpand(true);
    app_lbl.set_halign(gtk4::Align::Start);
    let close_btn = Button::with_label("✕");
    close_btn.add_css_class("toast-close");
    header.append(&app_lbl);
    header.append(&close_btn);

    let summary_lbl = Label::new(Some(summary));
    summary_lbl.add_css_class("toast-summary");
    summary_lbl.set_halign(gtk4::Align::Start);
    summary_lbl.set_wrap(true);
    summary_lbl.set_max_width_chars(38);

    toast.append(&header);
    toast.append(&summary_lbl);

    if !body.is_empty() {
        let body_lbl = Label::new(Some(body));
        body_lbl.add_css_class("toast-body");
        body_lbl.set_halign(gtk4::Align::Start);
        body_lbl.set_wrap(true);
        body_lbl.set_max_width_chars(38);
        toast.append(&body_lbl);
    }

    toast
}

// ── Notification centre popover ───────────────────────────────────────────────

/// Build the Popover opened by the bell button.
/// The list is rebuilt fresh each time the popover is shown.
pub fn build_notifications_popover(state: Rc<RefCell<PanelState>>) -> Popover {
    load_css(NOTIF_CSS);

    let popover = Popover::new();
    popover.set_has_arrow(false);
    popover.set_position(gtk4::PositionType::Bottom);

    let root = GtkBox::new(Orientation::Vertical, 6);
    root.set_margin_top(8);
    root.set_margin_bottom(8);
    root.set_margin_start(12);
    root.set_margin_end(12);
    root.set_size_request(260, -1);

    let title = Label::new(Some("Notifications"));
    title.add_css_class("nc-title");
    title.set_halign(gtk4::Align::Start);
    root.append(&title);

    let scroll = ScrolledWindow::new();
    scroll.set_max_content_height(320);
    scroll.set_propagate_natural_height(true);
    scroll.set_has_frame(false);

    let list = ListBox::new();
    list.set_show_separators(true);
    list.set_selection_mode(gtk4::SelectionMode::None);
    scroll.set_child(Some(&list));
    root.append(&scroll);
    popover.set_child(Some(&root));

    // Live-refresh: rebuild the list whenever the popover opens.
    let list_c = list.clone();
    let state_c = Rc::clone(&state);
    popover.connect_show(move |_| {
        rebuild_list(&list_c, &state_c);
    });

    popover
}

fn rebuild_list(list: &ListBox, state: &Rc<RefCell<PanelState>>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let s = state.borrow();
    if s.notifications.is_empty() {
        let empty = Label::new(Some("No notifications"));
        empty.add_css_class("notif-empty");
        empty.set_halign(gtk4::Align::Center);
        list.append(&make_row(&empty));
    } else {
        for item in s.notifications.iter().rev().take(20) {
            let row_box = GtkBox::new(Orientation::Vertical, 2);
            row_box.add_css_class("notif-row");

            let header = GtkBox::new(Orientation::Horizontal, 0);
            let app_lbl = Label::new(Some(&item.app));
            app_lbl.add_css_class("notif-app");
            app_lbl.set_hexpand(true);
            app_lbl.set_halign(gtk4::Align::Start);

            let dismiss_btn = Button::with_label("✕");
            dismiss_btn.add_css_class("notif-dismiss");
            dismiss_btn.set_halign(gtk4::Align::End);

            header.append(&app_lbl);
            header.append(&dismiss_btn);

            let summary_lbl = Label::new(Some(&item.summary));
            summary_lbl.add_css_class("notif-summary");
            summary_lbl.set_halign(gtk4::Align::Start);
            summary_lbl.set_wrap(true);
            row_box.append(&header);
            row_box.append(&summary_lbl);

            if !item.body.is_empty() {
                let body_lbl = Label::new(Some(&item.body));
                body_lbl.add_css_class("notif-body");
                body_lbl.set_halign(gtk4::Align::Start);
                body_lbl.set_wrap(true);
                row_box.append(&body_lbl);
            }

            let row = make_row(&row_box);
            {
                let id = item.id;
                let state_d = Rc::clone(state);
                let list_d = list.clone();
                let row_c = row.clone();
                dismiss_btn.connect_clicked(move |_| {
                    let _ = state_d.borrow_mut().dismiss_notification(id);
                    list_d.remove(&row_c);
                });
            }
            list.append(&row);
        }
    }
}

fn make_row(child: &impl IsA<gtk4::Widget>) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_child(Some(child));
    row
}

// GTK4 UI for the niri status panel.
// Zero business logic — all state types sourced from `crate::panel_backend`.

use std::{cell::RefCell, rc::Rc};

use gtk4::{
    glib,
    prelude::*,
    Application, ApplicationWindow, Box as GtkBox, Button, EventControllerScroll,
    EventControllerScrollFlags, Image, Label, ListBox, ListBoxRow, Orientation, Popover,
    ProgressBar, ScrolledWindow, Separator,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::panel_backend::PanelState;

const CSS: &str = r#"
window {
    background: transparent;
}

.panel-bar {
    background: rgba(15, 15, 25, 0.82);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 10px;
    padding: 4px 14px;
    font-family: "Inter", "Noto Sans", sans-serif;
}

.sb-pill {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 6px;
    padding: 3px 8px;
}

.ws-dot {
    min-width: 7px;
    min-height: 7px;
    border-radius: 50%;
    background: #565f89;
}
.ws-dot-active   { background: #7aa2f7; }
.ws-dot-occupied { background: #3d59a1; }

.media-btn {
    min-width: 16px;
    min-height: 16px;
    padding: 1px 3px;
    background: none;
    border: none;
    border-radius: 3px;
    color: #7aa2f7;
    font-size: 10px;
}
.media-btn:hover { background: rgba(122, 162, 247, 0.15); }

.media-track {
    font-size: 11px;
    color: #c0caf5;
}

.pill-label  { font-size: 10px; color: #565f89; }
.net-up      { font-size: 11px; color: #9ece6a; }
.net-down    { font-size: 11px; color: #7dcfff; }
.cpu-pct     { font-size: 10px; color: #e0af68; }
.mem-pct     { font-size: 10px; color: #bb9af7; }
.vol-pct     { font-size: 11px; color: #c0caf5; }
.time-label  { font-size: 13px; font-weight: bold; color: #c0caf5; }
.date-label  { font-size: 9px;  color: #565f89; }
.weather-temp { font-size: 11px; color: #c0caf5; }
.weather-loc  { font-size: 10px; color: #565f89; }

progressbar.cpu-bar,
progressbar.mem-bar {
    min-width: 32px;
}
progressbar.cpu-bar trough,
progressbar.mem-bar trough {
    min-height: 6px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.10);
}
progressbar.cpu-bar progress { background: #e0af68; border-radius: 3px; }
progressbar.mem-bar progress { background: #bb9af7; border-radius: 3px; }

.icon-btn {
    min-width: 24px;
    min-height: 24px;
    padding: 0 5px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 6px;
    color: #c0caf5;
    font-size: 13px;
}
.icon-btn:hover { background: rgba(255, 255, 255, 0.12); }

.qs-row   { padding: 4px 0; }
.qs-title { font-size: 12px; font-weight: bold; color: #c0caf5; margin-bottom: 6px; }
.qs-label { font-size: 12px; color: #c0caf5; }

.notif-row     { padding: 6px 4px; }
.notif-app     { font-size: 10px; color: #565f89; }
.notif-summary { font-size: 12px; color: #c0caf5; font-weight: bold; }
.notif-body    { font-size: 11px; color: #a9b1d6; }
.notif-dismiss {
    min-width: 18px;
    min-height: 18px;
    padding: 0 4px;
    background: rgba(247, 118, 142, 0.15);
    border: 1px solid rgba(247, 118, 142, 0.25);
    border-radius: 4px;
    color: #f7768e;
    font-size: 10px;
}
.notif-empty { font-size: 12px; color: #565f89; padding: 8px 4px; }
"#;

/// Called once inside `app.connect_activate`. Loads CSS and builds the panel window.
pub fn build_panel_window(app: &Application, state: Rc<RefCell<PanelState>>) {
    load_css();
    build_window(app, state);
}

fn load_css() {
    let Some(display) = gtk4::gdk::Display::default() else {
        log::warn!("panel_ui: no GDK display, skipping CSS load");
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

fn build_window(app: &Application, state: Rc<RefCell<PanelState>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("niri-panel")
        .decorated(false)
        .build();

    // Pin to the top edge of the output, full width, above all windows.
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.auto_exclusive_zone_enable();
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Bottom, false);

    let bar = GtkBox::new(Orientation::Horizontal, 0);
    bar.add_css_class("panel-bar");
    bar.set_hexpand(true);
    bar.set_valign(gtk4::Align::Center);

    // ── Left: workspace dots + media controls ────────────────────────────────
    let left = GtkBox::new(Orientation::Horizontal, 10);
    left.set_valign(gtk4::Align::Center);
    left.set_margin_end(4);

    let ws_pill = GtkBox::new(Orientation::Horizontal, 4);
    ws_pill.add_css_class("sb-pill");
    let ws_dots: Vec<GtkBox> = (0..5)
        .map(|i| {
            let d = GtkBox::new(Orientation::Horizontal, 0);
            d.set_size_request(7, 7);
            d.set_hexpand(false);
            d.set_vexpand(false);
            d.set_halign(gtk4::Align::Center);
            d.set_valign(gtk4::Align::Center);
            d.add_css_class("ws-dot");
            if i == 0 {
                d.add_css_class("ws-dot-active");
            }
            ws_pill.append(&d);
            d
        })
        .collect();
    // Scroll wheel on workspace dots to switch workspaces.
    {
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll(|_ctrl, _dx, dy| {
            let action =
                if dy < 0.0 { "focus-workspace-up" } else { "focus-workspace-down" };
            let _ = std::process::Command::new("niri")
                .args(["msg", "action", action])
                .spawn();
            glib::Propagation::Stop
        });
        ws_pill.add_controller(scroll);
    }
    left.append(&ws_pill);

    let media_pill = GtkBox::new(Orientation::Horizontal, 5);
    media_pill.add_css_class("sb-pill");

    let prev_btn = Button::new();
    {
        let img = Image::from_icon_name("media-skip-backward-symbolic");
        img.set_pixel_size(12);
        prev_btn.set_child(Some(&img));
    }
    prev_btn.add_css_class("media-btn");
    prev_btn.connect_clicked(|_| {
        std::thread::spawn(|| { crate::media::send_previous(); });
    });

    let play_icon = Image::from_icon_name("media-playback-start-symbolic");
    play_icon.set_pixel_size(12);
    let play_btn = Button::new();
    play_btn.set_child(Some(&play_icon));
    play_btn.add_css_class("media-btn");
    play_btn.connect_clicked(|_| {
        std::thread::spawn(|| { crate::media::send_play_pause(); });
    });

    let next_btn = Button::new();
    {
        let img = Image::from_icon_name("media-skip-forward-symbolic");
        img.set_pixel_size(12);
        next_btn.set_child(Some(&img));
    }
    next_btn.add_css_class("media-btn");
    next_btn.connect_clicked(|_| {
        std::thread::spawn(|| { crate::media::send_next(); });
    });

    let track_label = Label::new(Some("No media"));
    track_label.add_css_class("media-track");
    track_label.set_max_width_chars(20);
    track_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    media_pill.append(&prev_btn);
    media_pill.append(&play_btn);
    media_pill.append(&next_btn);
    media_pill.append(&track_label);
    left.append(&media_pill);

    bar.append(&left);

    // ── Spacer ───────────────────────────────────────────────────────────────
    let spacer1 = GtkBox::new(Orientation::Horizontal, 0);
    spacer1.set_hexpand(true);
    bar.append(&spacer1);

    // ── Center: weather pill + time/date block ────────────────────────────────
    let center = GtkBox::new(Orientation::Horizontal, 10);
    center.set_valign(gtk4::Align::Center);

    let weather_pill = GtkBox::new(Orientation::Horizontal, 5);
    weather_pill.add_css_class("sb-pill");
    let weather_icon = Label::new(Some("⛅"));
    let weather_temp = Label::new(Some("—°C"));
    weather_temp.add_css_class("weather-temp");
    let weather_loc = Label::new(Some("—"));
    weather_loc.add_css_class("weather-loc");
    weather_pill.append(&weather_icon);
    weather_pill.append(&weather_temp);
    weather_pill.append(&weather_loc);
    center.append(&weather_pill);

    let time_box = GtkBox::new(Orientation::Vertical, 0);
    time_box.set_valign(gtk4::Align::Center);
    let time_label = Label::new(Some("00:00"));
    time_label.add_css_class("time-label");
    let date_label = Label::new(Some(""));
    date_label.add_css_class("date-label");
    time_box.append(&time_label);
    time_box.append(&date_label);
    center.append(&weather_pill);
    // time is placed in the right section, next to net stats

    bar.append(&center);

    // ── Spacer ───────────────────────────────────────────────────────────────
    let spacer2 = GtkBox::new(Orientation::Horizontal, 0);
    spacer2.set_hexpand(true);
    bar.append(&spacer2);

    // ── Right: time + stats + quick settings + notifications ──────────────────
    let right = GtkBox::new(Orientation::Horizontal, 6);
    right.set_valign(gtk4::Align::Center);
    right.set_margin_start(4);

    // Time/date — sits right before network stats
    let time_sep = Separator::new(Orientation::Vertical);
    time_sep.set_margin_start(2);
    time_sep.set_margin_end(2);
    right.append(&time_box);
    right.append(&time_sep);

    // Network pill
    let net_pill = GtkBox::new(Orientation::Horizontal, 4);
    net_pill.add_css_class("sb-pill");
    let net_up_arrow = Label::new(Some("↑"));
    net_up_arrow.add_css_class("net-up");
    let net_up_val = Label::new(Some("0 B/s"));
    net_up_val.add_css_class("net-up");
    let net_dn_arrow = Label::new(Some("↓"));
    net_dn_arrow.add_css_class("net-down");
    let net_dn_val = Label::new(Some("0 B/s"));
    net_dn_val.add_css_class("net-down");
    net_pill.append(&net_up_arrow);
    net_pill.append(&net_up_val);
    net_pill.append(&net_dn_arrow);
    net_pill.append(&net_dn_val);
    right.append(&net_pill);

    // CPU pill
    let cpu_pill = GtkBox::new(Orientation::Horizontal, 4);
    cpu_pill.add_css_class("sb-pill");
    let cpu_tag = Label::new(Some("CPU"));
    cpu_tag.add_css_class("pill-label");
    let cpu_bar = ProgressBar::new();
    cpu_bar.add_css_class("cpu-bar");
    cpu_bar.set_show_text(false);
    cpu_bar.set_size_request(32, 6);
    cpu_bar.set_valign(gtk4::Align::Center);
    let cpu_pct_lbl = Label::new(Some("0%"));
    cpu_pct_lbl.add_css_class("cpu-pct");
    cpu_pill.append(&cpu_tag);
    cpu_pill.append(&cpu_bar);
    cpu_pill.append(&cpu_pct_lbl);
    right.append(&cpu_pill);

    // MEM pill
    let mem_pill = GtkBox::new(Orientation::Horizontal, 4);
    mem_pill.add_css_class("sb-pill");
    let mem_tag = Label::new(Some("MEM"));
    mem_tag.add_css_class("pill-label");
    let mem_bar = ProgressBar::new();
    mem_bar.add_css_class("mem-bar");
    mem_bar.set_show_text(false);
    mem_bar.set_size_request(32, 6);
    mem_bar.set_valign(gtk4::Align::Center);
    let mem_pct_lbl = Label::new(Some("0G"));
    mem_pct_lbl.add_css_class("mem-pct");
    mem_pill.append(&mem_tag);
    mem_pill.append(&mem_bar);
    mem_pill.append(&mem_pct_lbl);
    right.append(&mem_pill);

    // Volume pill
    let vol_pill = GtkBox::new(Orientation::Horizontal, 4);
    vol_pill.add_css_class("sb-pill");
    let vol_icon = Label::new(Some("🔊"));
    let vol_pct_lbl = Label::new(Some("0%"));
    vol_pct_lbl.add_css_class("vol-pct");
    vol_pill.append(&vol_icon);
    vol_pill.append(&vol_pct_lbl);
    // Scroll wheel adjusts volume ±5%.
    {
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll(|_ctrl, _dx, dy| {
            let delta: i8 = if dy < 0.0 { 5 } else { -5 };
            crate::sysinfo::set_volume_delta(delta);
            glib::Propagation::Stop
        });
        vol_pill.add_controller(scroll);
    }
    right.append(&vol_pill);

    // Notifications button + popover
    let notif_btn = Button::with_label("🔔");
    notif_btn.add_css_class("icon-btn");
    let notif_popover = build_notifications_popover(Rc::clone(&state));
    notif_popover.set_parent(&notif_btn);
    notif_btn.connect_clicked(move |_| notif_popover.popup());
    right.append(&notif_btn);

    bar.append(&right);

    window.set_child(Some(&bar));
    window.present();

    // ── Timer: refresh display every 2 seconds ────────────────────────────────
    tick_time(&time_label, &date_label);

    let time_lbl_c = time_label.clone();
    let date_lbl_c = date_label.clone();
    let net_up_c = net_up_val.clone();
    let net_dn_c = net_dn_val.clone();
    let cpu_bar_c = cpu_bar.clone();
    let cpu_pct_c = cpu_pct_lbl.clone();
    let mem_bar_c = mem_bar.clone();
    let mem_pct_c = mem_pct_lbl.clone();
    let vol_c = vol_pct_lbl.clone();
    let track_c = track_label.clone();
    let play_icon_c = play_icon.clone();
    let wtemp_c = weather_temp.clone();
    let wloc_c = weather_loc.clone();
    let state_c = Rc::clone(&state);

    // Fast 200 ms timer — only updates workspace dots so they feel instant.
    {
        let dots = ws_dots.clone();
        let state_fast = Rc::clone(&state);
        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            let s = state_fast.borrow();
            let ws = &s.workspaces;
            for (i, dot) in dots.iter().enumerate() {
                dot.remove_css_class("ws-dot-active");
                dot.remove_css_class("ws-dot-occupied");
                if i < ws.names.len() {
                    if i == ws.current_index {
                        dot.add_css_class("ws-dot-active");
                    } else if ws.occupied.get(i).copied().unwrap_or(false) {
                        dot.add_css_class("ws-dot-occupied");
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    glib::timeout_add_seconds_local(2, move || {
        tick_time(&time_lbl_c, &date_lbl_c);

        // Pull live system stats and write them into shared state.
        if let Some(snap) = crate::sysinfo::sample() {
            let mut s = state_c.borrow_mut();
            let _ = s.update_stats(
                snap.cpu_percent,
                snap.memory_used,
                snap.memory_total,
                snap.net_up,
                snap.net_down,
                snap.volume,
            );
        }

        // Pull MPRIS media (non-blocking D-Bus poll).
        if let Some(m) = crate::media::poll_media() {
            let mut s = state_c.borrow_mut();
            let _ = s.update_media(m.title.clone(), m.artist.clone(), m.playing);
        }

        let s = state_c.borrow();

        // Media
        if s.media.playing {
            play_icon_c.set_icon_name(Some("media-playback-pause-symbolic"));
        } else {
            play_icon_c.set_icon_name(Some("media-playback-start-symbolic"));
        }
        if !s.media.track_name.is_empty() {
            track_c.set_text(&format!("{} — {}", s.media.artist, s.media.track_name));
        }

        // Weather
        if !s.weather.location.is_empty() {
            wtemp_c.set_text(&format!("{:.0}°C", s.weather.temperature));
            wloc_c.set_text(&s.weather.location);
        }

        // Network
        net_up_c.set_text(&fmt_speed(s.stats.network_up));
        net_dn_c.set_text(&fmt_speed(s.stats.network_down));

        // CPU
        cpu_bar_c.set_fraction((s.stats.cpu_percent / 100.0).clamp(0.0, 1.0) as f64);
        cpu_pct_c.set_text(&format!("{:.0}%", s.stats.cpu_percent));

        // MEM
        let mem_ratio = if s.stats.memory_total > 0 {
            s.stats.memory_used as f64 / s.stats.memory_total as f64
        } else {
            0.0
        };
        mem_bar_c.set_fraction(mem_ratio.clamp(0.0, 1.0));
        mem_pct_c.set_text(&format!(
            "{:.1}G",
            s.stats.memory_used as f64 / 1_073_741_824.0
        ));

        // Volume
        vol_c.set_text(&format!("{}%", s.stats.volume_percent));

        glib::ControlFlow::Continue
    });
}

fn build_notifications_popover(state: Rc<RefCell<PanelState>>) -> Popover {
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
    title.add_css_class("qs-title");
    title.set_halign(gtk4::Align::Start);
    root.append(&title);

    let scroll = ScrolledWindow::new();
    scroll.set_max_content_height(300);
    scroll.set_propagate_natural_height(true);
    scroll.set_has_frame(false);

    let list = ListBox::new();
    list.set_show_separators(true);
    list.set_selection_mode(gtk4::SelectionMode::None);

    if state.borrow().notifications.is_empty() {
        let empty = Label::new(Some("No notifications"));
        empty.add_css_class("notif-empty");
        empty.set_halign(gtk4::Align::Center);
        list.append(&make_notif_row_widget(&empty));
    } else {
        for item in &state.borrow().notifications {
            let row_box = GtkBox::new(Orientation::Vertical, 2);
            row_box.add_css_class("notif-row");

            let app_lbl = Label::new(Some(&item.app));
            app_lbl.add_css_class("notif-app");
            app_lbl.set_halign(gtk4::Align::Start);

            let summary_lbl = Label::new(Some(&item.summary));
            summary_lbl.add_css_class("notif-summary");
            summary_lbl.set_halign(gtk4::Align::Start);
            summary_lbl.set_wrap(true);

            let body_lbl = Label::new(Some(&item.body));
            body_lbl.add_css_class("notif-body");
            body_lbl.set_halign(gtk4::Align::Start);
            body_lbl.set_wrap(true);

            let header = GtkBox::new(Orientation::Horizontal, 0);
            header.append(&app_lbl);
            let dismiss_btn = Button::with_label("✕");
            dismiss_btn.add_css_class("notif-dismiss");
            dismiss_btn.set_hexpand(true);
            dismiss_btn.set_halign(gtk4::Align::End);
            let id = item.id;
            let state_c = Rc::clone(&state);
            dismiss_btn.connect_clicked(move |_| {
                let _ = state_c.borrow_mut().dismiss_notification(id);
                log::info!("dismissed notification {id}");
            });
            header.append(&dismiss_btn);

            row_box.append(&header);
            row_box.append(&summary_lbl);
            if !item.body.is_empty() {
                row_box.append(&body_lbl);
            }

            let row = ListBoxRow::new();
            row.set_child(Some(&row_box));
            list.append(&row);
        }
    }

    scroll.set_child(Some(&list));
    root.append(&scroll);
    popover.set_child(Some(&root));
    popover
}

fn make_notif_row_widget(child: &impl IsA<gtk4::Widget>) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_child(Some(child));
    row
}

fn tick_time(time_lbl: &Label, date_lbl: &Label) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (h, m) = ((secs / 3600) % 24, (secs / 60) % 60);
    time_lbl.set_text(&format!("{h:02}:{m:02}"));
    let days_since_epoch = secs / 86400;
    let dow =
        ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][(days_since_epoch % 7) as usize];
    let days_in_year = days_since_epoch % 365;
    let (month, dom) = approx_month_day(days_in_year);
    date_lbl.set_text(&format!("{dow}, {month} {dom}"));
}

fn approx_month_day(day: u64) -> (&'static str, u64) {
    const MONTHS: [(&str, u64); 12] = [
        ("Jan", 31),
        ("Feb", 28),
        ("Mar", 31),
        ("Apr", 30),
        ("May", 31),
        ("Jun", 30),
        ("Jul", 31),
        ("Aug", 31),
        ("Sep", 30),
        ("Oct", 31),
        ("Nov", 30),
        ("Dec", 31),
    ];
    let mut remaining = day;
    for (name, days) in MONTHS {
        if remaining < days {
            return (name, remaining + 1);
        }
        remaining -= days;
    }
    ("Dec", 31)
}

fn fmt_speed(bps: u64) -> String {
    if bps >= 1_048_576 {
        format!("{:.1} MB/s", bps as f64 / 1_048_576.0)
    } else if bps >= 1024 {
        format!("{:.0} KB/s", bps as f64 / 1024.0)
    } else {
        format!("{bps} B/s")
    }
}

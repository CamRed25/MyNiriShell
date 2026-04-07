// GTK4 UI for the niri status panel.
// Zero business logic — all state types sourced from `crate::panel_backend`.

use std::{cell::RefCell, rc::Rc};

use gtk4::{
    glib,
    prelude::*,
    Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, EventControllerScroll,
    EventControllerScrollFlags, Image, Label, Orientation, ProgressBar, Separator,
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

.scratch-badge {
    font-size: 8px;
    color: #bb9af7;
    background: rgba(187, 154, 247, 0.15);
    border: 1px solid rgba(187, 154, 247, 0.30);
    border-radius: 4px;
    padding: 1px 5px;
    margin-left: 4px;
}

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

progressbar.mem-bar {
    min-width: 32px;
}
progressbar.mem-bar trough {
    min-height: 6px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.10);
}
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



.cal-btn {
    background: none;
    border: none;
    padding: 2px 4px;
    border-radius: 6px;
}
.cal-btn:hover { background: rgba(255, 255, 255, 0.07); }

.cal-header {
    font-size: 11px;
    font-weight: bold;
    color: #7aa2f7;
    letter-spacing: 0.04em;
}
.cal-dow {
    font-size: 9px;
    color: #565f89;
    min-width: 22px;
    min-height: 16px;
}
.cal-day {
    font-size: 10px;
    color: #c0caf5;
    font-family: "JetBrains Mono", monospace;
    min-width: 22px;
    min-height: 20px;
    border-radius: 4px;
}
.cal-day.today {
    background: rgba(122, 162, 247, 0.20);
    color: #7aa2f7;
    font-weight: bold;
}
.cal-day.empty { color: transparent; }
"#;

/// Called once inside `app.connect_activate`. Loads CSS and builds the panel window.
pub fn build_panel_window(
    app: &Application,
    state: Rc<RefCell<PanelState>>,
    osd: Rc<crate::osd_ui::OsdWindow>,
    qs_win: Rc<crate::quick_settings_ui::QuickSettingsWindow>,
) {
    load_css();
    build_window(app, state, osd, qs_win);
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

fn build_window(
    app: &Application,
    state: Rc<RefCell<PanelState>>,
    osd: Rc<crate::osd_ui::OsdWindow>,
    qs_win: Rc<crate::quick_settings_ui::QuickSettingsWindow>,
) {
    let cpu_history: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(Vec::with_capacity(20)));
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

    // Scratchpad badge — shown when niri has hidden scratchpad windows.
    let scratch_badge = Label::new(Some("⦿"));
    scratch_badge.add_css_class("scratch-badge");
    scratch_badge.set_visible(false);
    left.append(&scratch_badge);

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

    // Wrap time_box in a button that opens a calendar popover on click.
    let cal_btn = Button::new();
    cal_btn.add_css_class("cal-btn");
    cal_btn.set_child(Some(&time_box));
    let cal_popover = build_calendar_popover();
    cal_popover.set_parent(&cal_btn);
    cal_btn.connect_clicked(move |_| cal_popover.popup());

    center.append(&cal_btn);
    bar.append(&center);

    // ── Spacer ───────────────────────────────────────────────────────────────
    let spacer2 = GtkBox::new(Orientation::Horizontal, 0);
    spacer2.set_hexpand(true);
    bar.append(&spacer2);

    // ── Right: time + stats + quick settings + notifications ──────────────────
    let right = GtkBox::new(Orientation::Horizontal, 6);
    right.set_valign(gtk4::Align::Center);
    right.set_margin_start(4);

    // Thin separator before network stats
    let time_sep = Separator::new(Orientation::Vertical);
    time_sep.set_margin_start(2);
    time_sep.set_margin_end(2);
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
    let cpu_draw = DrawingArea::new();
    cpu_draw.set_size_request(44, 14);
    cpu_draw.set_valign(gtk4::Align::Center);
    {
        let hist = Rc::clone(&cpu_history);
        cpu_draw.set_draw_func(move |_area, cr, w, h| {
            let buf = hist.borrow();
            let n = buf.len();
            if n < 2 {
                return;
            }
            let wf = w as f64;
            let hf = h as f64;
            let step = wf / (n as f64 - 1.0);
            // Fill under the curve.
            cr.set_source_rgba(0.878, 0.686, 0.408, 0.15);
            cr.move_to(0.0, hf);
            for (i, &v) in buf.iter().enumerate() {
                cr.line_to(i as f64 * step, hf - (v / 100.0) as f64 * hf);
            }
            cr.line_to((n - 1) as f64 * step, hf);
            cr.close_path();
            let _ = cr.fill();
            // Draw the line.
            cr.set_source_rgba(0.878, 0.686, 0.408, 0.9);
            cr.set_line_width(1.2);
            cr.move_to(0.0, hf - (buf[0] / 100.0) as f64 * hf);
            for (i, &v) in buf.iter().enumerate().skip(1) {
                cr.line_to(i as f64 * step, hf - (v / 100.0) as f64 * hf);
            }
            let _ = cr.stroke();
        });
    }
    let cpu_pct_lbl = Label::new(Some("0%"));
    cpu_pct_lbl.add_css_class("cpu-pct");
    cpu_pill.append(&cpu_tag);
    cpu_pill.append(&cpu_draw);
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
        let osd_c = Rc::clone(&osd);
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll(move |_ctrl, _dx, dy| {
            let delta: i8 = if dy < 0.0 { 5 } else { -5 };
            let new_vol = crate::sysinfo::set_volume_delta(delta);
            osd_c.show("🔊", new_vol);
            glib::Propagation::Stop
        });
        vol_pill.add_controller(scroll);
    }
    right.append(&vol_pill);

    // Quick Settings button
    let qs_win_c = Rc::clone(&qs_win);
    let qs_btn = Button::with_label("⚙");
    qs_btn.add_css_class("icon-btn");
    qs_btn.connect_clicked(move |_| qs_win_c.toggle());
    right.append(&qs_btn);

    // Notifications button + popover (list rebuilt live on each open)
    let notif_btn = Button::with_label("🔔");
    notif_btn.add_css_class("icon-btn");
    let notif_popover = crate::notification_ui::build_notifications_popover(Rc::clone(&state));
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
    let cpu_draw_c = cpu_draw.clone();
    let cpu_hist_c = Rc::clone(&cpu_history);
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
        let scratch_c = scratch_badge.clone();
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
            let n = ws.scratchpad_count;
            scratch_c.set_visible(n > 0);
            if n > 0 {
                scratch_c.set_text(&format!("⦿ {n}"));
            }
            glib::ControlFlow::Continue
        });
    }

    glib::timeout_add_seconds_local(2, move || {
        tick_time(&time_lbl_c, &date_lbl_c);

        // Pull live system stats and write them into shared state.
        if let Some(snap) = crate::sysinfo::sample() {
            {
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
            let mut hist = cpu_hist_c.borrow_mut();
            hist.push(snap.cpu_percent);
            if hist.len() > 20 {
                hist.remove(0);
            }
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

        // CPU sparkline
        cpu_draw_c.queue_draw();
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

// ── Calendar popover ─────────────────────────────────────────────────────────

fn build_calendar_popover() -> gtk4::Popover {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as libc::time_t;
    // SAFETY: localtime_r is reentrant; `now` is a valid time_t value.
    let mut ltm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&now, &mut ltm) };
    let year = ltm.tm_year + 1900;
    let month = (ltm.tm_mon + 1) as u32;
    let day = ltm.tm_mday as u32;
    let month_name = MONTH_NAMES[(month - 1) as usize];
    let first_weekday = weekday_of(year, month, 1); // 0=Mon…6=Sun

    let popover = gtk4::Popover::new();
    popover.set_has_arrow(true);
    popover.set_position(gtk4::PositionType::Bottom);

    let root = GtkBox::new(Orientation::Vertical, 6);
    root.set_margin_top(10);
    root.set_margin_bottom(10);
    root.set_margin_start(12);
    root.set_margin_end(12);

    // Month header
    let header = Label::new(Some(&format!("{month_name} {year}")));
    header.add_css_class("cal-header");
    header.set_halign(gtk4::Align::Center);
    root.append(&header);

    // Grid: 7 columns (Mon–Sun)
    let grid = gtk4::Grid::new();
    grid.set_row_spacing(2);
    grid.set_column_spacing(2);
    grid.set_halign(gtk4::Align::Center);

    for (col, dow) in ["Mo","Tu","We","Th","Fr","Sa","Su"].iter().enumerate() {
        let lbl = Label::new(Some(dow));
        lbl.add_css_class("cal-dow");
        lbl.set_halign(gtk4::Align::Center);
        grid.attach(&lbl, col as i32, 0, 1, 1);
    }

    let days_in_month = days_in(year, month);
    // first_weekday: 0 = Mon, col 0; 6 = Sun, col 6
    let mut col = first_weekday as i32;
    let mut grid_row = 1i32;

    for d in 1..=(days_in_month as i32) {
        let lbl = Label::new(Some(&format!("{d}")));
        lbl.add_css_class("cal-day");
        lbl.set_halign(gtk4::Align::Center);
        if d == day as i32 {
            lbl.add_css_class("today");
        }
        grid.attach(&lbl, col, grid_row, 1, 1);
        col += 1;
        if col == 7 {
            col = 0;
            grid_row += 1;
        }
    }

    root.append(&grid);
    popover.set_child(Some(&root));
    popover
}

const MONTH_NAMES: [&str; 12] = [
    "January","February","March","April","May","June",
    "July","August","September","October","November","December",
];


/// Weekday of (year, month, day): 0 = Monday, 6 = Sunday (ISO).
fn weekday_of(year: i32, month: u32, day: u32) -> u32 {
    // Tomohiko Sakamoto's algorithm adapted to Mon=0.
    let t: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let d = day as i32;
    let dow = (y + y/4 - y/100 + y/400 + t[(month-1) as usize] + d) % 7;
    // dow is Sun=0 → convert to Mon=0 ISO
    ((dow + 6) % 7) as u32
}

/// Number of days in (year, month).
fn days_in(year: i32, month: u32) -> u32 {
    match month {
        1|3|5|7|8|10|12 => 31,
        4|6|9|11 => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 29 } else { 28 },
        _ => 30,
    }
}

fn tick_time(time_lbl: &Label, date_lbl: &Label) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as libc::time_t;
    // SAFETY: localtime_r is reentrant; `now` is a valid time_t value.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&now, &mut tm) };
    time_lbl.set_text(&format!("{:02}:{:02}", tm.tm_hour, tm.tm_min));
    const DOW: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MON: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                              "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    date_lbl.set_text(&format!(
        "{}, {} {}",
        DOW[tm.tm_wday as usize],
        MON[tm.tm_mon as usize],
        tm.tm_mday,
    ));
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

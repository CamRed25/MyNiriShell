// GTK4 UI for niri-launcher.
// Zero backend logic — all business logic lives in launcher_backend.rs.

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, CssProvider, Entry,
    EventControllerKey, Image, Label, ListBox, ListBoxRow, Orientation, Separator,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::launcher_backend::{
    evaluate_expression, fuzzy_search, launch_app, load_apps, CalcResult, SearchResult,
};

const WINDOW_WIDTH: i32 = 360;
const MAX_RESULTS: usize = 8;

/// Set by the SIGUSR1 handler; polled by the GTK main-thread timer.
static TOGGLE_REQUESTED: AtomicBool = AtomicBool::new(false);

/// POSIX signal handler — only async-signal-safe operations allowed.
extern "C" fn sigusr1_handler(_: libc::c_int) {
    TOGGLE_REQUESTED.store(true, Ordering::Relaxed);
}

const CSS: &str = r#"
* { box-shadow: none; }

window {
    background: rgba(13, 13, 23, 0.93);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: #c0caf5;
    font-family: "Inter", sans-serif;
    font-size: 13px;
}

.launcher-root {
    background: transparent;
}

.search-row {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    padding: 7px 10px;
    margin-bottom: 12px;
}

.search-icon { color: #565f89; }

entry {
    background: transparent;
    border: none;
    outline: none;
    color: #c0caf5;
    font-size: 13px;
    font-family: "Inter", sans-serif;
    box-shadow: none;
    min-height: 0;
    padding: 0;
}
entry:focus { box-shadow: none; }
entry > text { color: #c0caf5; }
entry > text > selection { background: rgba(122, 162, 247, 0.35); }
entry placeholder { color: #565f89; }

.calc-badge {
    font-size: 10px;
    color: #9ece6a;
    background: rgba(158, 206, 106, 0.12);
    border: 1px solid rgba(158, 206, 106, 0.25);
    border-radius: 4px;
    padding: 2px 6px;
}

.section-label {
    font-size: 10px;
    color: #565f89;
    letter-spacing: 0.06em;
    padding: 0 2px;
    margin-bottom: 4px;
}

listbox { background: transparent; }
listbox > row {
    background: transparent;
    padding: 0;
    border-radius: 7px;
}
listbox > row:hover { background: transparent; }
listbox > row:selected { background: transparent; }

.result-item {
    border-radius: 7px;
    padding: 6px 8px;
    background: transparent;
}
.result-item.selected { background: rgba(122, 162, 247, 0.14); }

.result-name {
    font-size: 13px;
    color: #c0caf5;
}
.result-sub {
    font-size: 10px;
    color: #565f89;
}

separator.divider {
    background: rgba(255, 255, 255, 0.07);
    min-height: 1px;
    margin: 10px 0;
}

.calc-area { padding: 4px 2px; }

.calc-expr {
    font-size: 12px;
    color: #7dcfff;
    font-family: "JetBrains Mono", monospace;
}
.calc-equals { font-size: 11px; color: #565f89; }
.calc-result-value {
    font-size: 16px;
    color: #9ece6a;
    font-family: "JetBrains Mono", monospace;
    font-weight: 500;
}

.high-contrast .result-name { color: #ffffff; }
.high-contrast .result-sub { color: #aaaaaa; }
.high-contrast .section-label { color: #bbbbbb; }
.high-contrast .result-item.selected { background: rgba(122, 162, 247, 0.35); }
.high-contrast entry > text { color: #ffffff; }
"#;

struct LauncherState {
    apps: Vec<crate::launcher_backend::AppEntry>,
    results: Vec<SearchResult>,
    selected: usize,
    calc: Option<CalcResult>,
}

/// Called once inside `app.connect_activate`. Loads CSS and builds the launcher window.
pub fn build_launcher_window(app: &Application) {
    build_window(app);
}

fn build_window(app: &Application) {
    let provider = CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let apps = load_apps();
    log::info!("niri-launcher: loaded {} app entries", apps.len());

    let initial_results = fuzzy_search("", &apps);

    let state = Rc::new(RefCell::new(LauncherState {
        apps,
        results: initial_results,
        selected: 0,
        calc: None,
    }));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("niri Launcher")
        .default_width(WINDOW_WIDTH)
        .resizable(false)
        .decorated(false)
        .build();

    // Place at screen centre using layer-shell overlay layer.
    // Keyboard mode: on-demand so it receives focus when shown.
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    // No edge anchoring → centred on screen.

    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(14)
        .margin_end(14)
        .accessible_role(gtk4::AccessibleRole::Group)
        .build();
    root.add_css_class("launcher-root");

    if is_high_contrast() {
        root.add_css_class("high-contrast");
    }

    let (search_row, entry, calc_badge) = build_search_row();
    root.append(&search_row);

    let apps_section = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .margin_bottom(10)
        .build();

    let apps_label = Label::builder()
        .label("apps")
        .halign(gtk4::Align::Start)
        .accessible_role(gtk4::AccessibleRole::Caption)
        .build();
    apps_label.add_css_class("section-label");
    apps_section.append(&apps_label);

    let list_box = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .accessible_role(gtk4::AccessibleRole::List)
        .build();
    list_box.set_activate_on_single_click(true);
    apps_section.append(&list_box);
    root.append(&apps_section);

    let divider = Separator::builder().orientation(Orientation::Horizontal).build();
    divider.add_css_class("divider");
    root.append(&divider);

    let calc_section = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .visible(false)
        .accessible_role(gtk4::AccessibleRole::Group)
        .build();

    let calc_label = Label::builder().label("calculator").halign(gtk4::Align::Start).build();
    calc_label.add_css_class("section-label");
    calc_section.append(&calc_label);

    let calc_area = GtkBox::builder().orientation(Orientation::Horizontal).spacing(8).build();
    calc_area.add_css_class("calc-area");

    let calc_expr_label = Label::builder()
        .halign(gtk4::Align::Start)
        .accessible_role(gtk4::AccessibleRole::Math)
        .build();
    calc_expr_label.add_css_class("calc-expr");
    calc_area.append(&calc_expr_label);

    let calc_equals = Label::builder().label("=").build();
    calc_equals.add_css_class("calc-equals");
    calc_area.append(&calc_equals);

    let calc_result_label = Label::builder()
        .halign(gtk4::Align::Start)
        .accessible_role(gtk4::AccessibleRole::Status)
        .build();
    calc_result_label.add_css_class("calc-result-value");
    calc_area.append(&calc_result_label);

    calc_section.append(&calc_area);
    root.append(&calc_section);

    window.set_child(Some(&root));

    {
        let st = state.borrow();
        update_results_list(&list_box, &st.results, st.selected);
    }

    // Entry changed → live search + calc
    {
        let state = Rc::clone(&state);
        let list_box = list_box.clone();
        let calc_badge = calc_badge.clone();
        let calc_section = calc_section.clone();
        let calc_expr_label = calc_expr_label.clone();
        let calc_result_label = calc_result_label.clone();

        entry.connect_changed(move |e| {
            let text = e.text().to_string();

            let calc = if !text.trim().is_empty() {
                let r = evaluate_expression(&text);
                if !r.is_error { Some(r) } else { None }
            } else {
                None
            };

            let results = {
                let st = state.borrow();
                fuzzy_search(&text, &st.apps)
            };

            let has_calc = calc.is_some();

            {
                let mut st = state.borrow_mut();
                st.results = results;
                st.selected = 0;
                st.calc = calc;
            }

            let st = state.borrow();

            calc_badge.set_visible(has_calc);

            if let Some(ref c) = st.calc {
                calc_section.set_visible(true);
                calc_expr_label.set_text(&c.expression);
                calc_result_label.set_text(&c.result);
            } else {
                calc_section.set_visible(false);
            }

            update_results_list(&list_box, &st.results, st.selected);
        });
    }

    // Click a row → launch that app
    {
        let state = Rc::clone(&state);
        let window_weak = window.downgrade();
        list_box.connect_row_activated(move |_lb, row| {
            let idx = row.index() as usize;
            let entry = state.borrow().results.get(idx).map(|r| r.entry.clone());
            if let Some(entry) = entry {
                match launch_app(&entry) {
                    Ok(()) => {
                        log::info!("Launched: {}", entry.name);
                        if let Some(w) = window_weak.upgrade() {
                            w.set_visible(false);
                        }
                    }
                    Err(e) => log::error!("Launch failed: {e}"),
                }
            }
        });
    }

    // Keyboard navigation
    // NOTE: Entry consumes Return itself and fires `activate` — the window
    // EventControllerKey never sees it. Wire entry.connect_activate for Enter.
    {
        let state = Rc::clone(&state);
        let window_weak = window.downgrade();
        entry.connect_activate(move |_e| {
            let (app_entry, calc_text) = {
                let st = state.borrow();
                let app = st.results.get(st.selected).map(|r| r.entry.clone());
                let calc = st.calc.as_ref().map(|c| c.result.clone());
                (app, calc)
            };
            if let Some(entry) = app_entry {
                match launch_app(&entry) {
                    Ok(()) => {
                        log::info!("Launched: {}", entry.name);
                        if let Some(w) = window_weak.upgrade() {
                            w.set_visible(false);
                        }
                    }
                    Err(e) => log::error!("Launch failed: {e}"),
                }
            } else if let Some(result_text) = calc_text {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&result_text);
                    log::info!("Copied calc result to clipboard: {result_text}");
                }
                if let Some(w) = window_weak.upgrade() {
                    w.set_visible(false);
                }
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let list_box = list_box.clone();
        let window_weak = window.downgrade();

        let key_ctrl = EventControllerKey::new();
        key_ctrl.connect_key_pressed(move |_ctrl, keyval, _code, _mods| {
            match keyval {
                gdk::Key::Escape => {
                    if let Some(w) = window_weak.upgrade() {
                        w.set_visible(false);
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Up => {
                    let (results, selected) = {
                        let mut st = state.borrow_mut();
                        if st.results.is_empty() {
                            return glib::Propagation::Stop;
                        }
                        st.selected = if st.selected == 0 {
                            st.results.len() - 1
                        } else {
                            st.selected - 1
                        };
                        (st.results.clone(), st.selected)
                    };
                    update_results_list(&list_box, &results, selected);
                    glib::Propagation::Stop
                }
                gdk::Key::Down => {
                    let (results, selected) = {
                        let mut st = state.borrow_mut();
                        if !st.results.is_empty() {
                            st.selected = (st.selected + 1) % st.results.len();
                        }
                        (st.results.clone(), st.selected)
                    };
                    update_results_list(&list_box, &results, selected);
                    glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    let (app_entry, calc_text) = {
                        let st = state.borrow();
                        let app = st.results.get(st.selected).map(|r| r.entry.clone());
                        let calc = st.calc.as_ref().map(|c| c.result.clone());
                        (app, calc)
                    };

                    if let Some(entry) = app_entry {
                        match launch_app(&entry) {
                            Ok(()) => {
                                log::info!("Launched: {}", entry.name);
                                if let Some(w) = window_weak.upgrade() {
                                    w.set_visible(false);
                                }
                            }
                            Err(e) => log::error!("Launch failed: {e}"),
                        }
                    } else if let Some(result_text) = calc_text {
                        if let Some(display) = gdk::Display::default() {
                            display.clipboard().set_text(&result_text);
                            log::info!("Copied calc result to clipboard: {result_text}");
                        }
                        if let Some(w) = window_weak.upgrade() {
                            w.set_visible(false);
                        }
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        window.add_controller(key_ctrl);
    }

    // Start hidden — SIGUSR1 toggles the launcher.
    // Niri config example: `bind "Super+Space" { spawn ["sh" "-c" "kill -USR1 $(pgrep niri-shell)"]; }`
    unsafe { libc::signal(libc::SIGUSR1, sigusr1_handler as *const () as libc::sighandler_t) };

    {
        let window_weak = window.downgrade();
        let entry_weak = entry.downgrade();
        let list_weak = list_box.downgrade();
        let state_sig = Rc::clone(&state);

        glib::timeout_add_local(Duration::from_millis(150), move || {
            if !TOGGLE_REQUESTED.swap(false, Ordering::Relaxed) {
                return glib::ControlFlow::Continue;
            }
            let Some(w) = window_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if w.is_visible() {
                w.set_visible(false);
            } else {
                // Reset search before presenting.
                {
                    let mut st = state_sig.borrow_mut();
                    st.selected = 0;
                    st.calc = None;
                    st.results = crate::launcher_backend::fuzzy_search("", &st.apps);
                }
                if let Some(e) = entry_weak.upgrade() {
                    e.set_text("");
                }
                if let Some(list) = list_weak.upgrade() {
                    let st = state_sig.borrow();
                    update_results_list(&list, &st.results, st.selected);
                }
                w.present();
                if let Some(e) = entry_weak.upgrade() {
                    e.grab_focus();
                }
            }
            glib::ControlFlow::Continue
        });
    }
}

fn build_search_row() -> (GtkBox, Entry, Label) {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .accessible_role(gtk4::AccessibleRole::SearchBox)
        .build();
    row.add_css_class("search-row");

    let icon = Image::builder()
        .icon_name("system-search-symbolic")
        .pixel_size(14)
        .accessible_role(gtk4::AccessibleRole::Img)
        .build();
    icon.add_css_class("search-icon");
    row.append(&icon);

    let entry = Entry::builder()
        .placeholder_text("search apps, files, commands…")
        .hexpand(true)
        .accessible_role(gtk4::AccessibleRole::SearchBox)
        .build();
    entry.set_has_frame(false);
    row.append(&entry);

    let calc_badge = Label::builder()
        .label("= calculator")
        .visible(false)
        .accessible_role(gtk4::AccessibleRole::Status)
        .build();
    calc_badge.add_css_class("calc-badge");
    row.append(&calc_badge);

    (row, entry, calc_badge)
}

fn update_results_list(list_box: &ListBox, results: &[SearchResult], selected: usize) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    for (i, result) in results.iter().take(MAX_RESULTS).enumerate() {
        let row = build_result_row(result, i == selected);
        list_box.append(&row);
    }
}

fn build_result_row(result: &SearchResult, selected: bool) -> ListBoxRow {
    let row = ListBoxRow::builder()
        .selectable(false)
        .activatable(true)
        .accessible_role(gtk4::AccessibleRole::ListItem)
        .build();
    row.set_widget_name(&result.entry.id);

    let item = GtkBox::builder().orientation(Orientation::Horizontal).spacing(10).build();
    item.add_css_class("result-item");
    if selected {
        item.add_css_class("selected");
    }

    let icon_name = if result.entry.icon.is_empty() {
        "application-x-executable-symbolic"
    } else {
        &result.entry.icon
    };
    let icon = Image::builder()
        .icon_name(icon_name)
        .pixel_size(20)
        .width_request(28)
        .height_request(28)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .accessible_role(gtk4::AccessibleRole::Img)
        .build();
    item.append(&icon);

    let text_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .build();

    let name_label = Label::builder()
        .halign(gtk4::Align::Start)
        .use_markup(true)
        .accessible_role(gtk4::AccessibleRole::Label)
        .build();
    name_label.add_css_class("result-name");
    name_label.set_markup(&build_match_markup(&result.entry.name, &result.match_ranges));
    text_col.append(&name_label);

    if !result.entry.description.is_empty() {
        let desc = Label::builder()
            .label(result.entry.description.as_str())
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .accessible_role(gtk4::AccessibleRole::Label)
            .build();
        desc.add_css_class("result-sub");
        text_col.append(&desc);
    }

    item.append(&text_col);
    row.set_child(Some(&item));
    row
}

fn build_match_markup(name: &str, ranges: &[(usize, usize)]) -> String {
    if ranges.is_empty() {
        return escape_markup(name);
    }

    let len = name.len();
    let mut out = String::with_capacity(len + ranges.len() * 32);
    let mut last = 0usize;

    for &(raw_start, raw_end) in ranges {
        // Clamp + ensure valid char boundaries to prevent any panic.
        let start = raw_start.min(len);
        let end = raw_end.min(len);
        if start >= end {
            continue;
        }
        // Walk start/end back to a valid UTF-8 char boundary.
        let start = (0..=start).rev().find(|&i| name.is_char_boundary(i)).unwrap_or(0);
        let end = (end..=len).find(|&i| name.is_char_boundary(i)).unwrap_or(len);

        if start > last {
            if let Some(s) = name.get(last..start) {
                out.push_str(&escape_markup(s));
            }
        }
        out.push_str("<span color=\"#7aa2f7\">");
        if let Some(s) = name.get(start..end) {
            out.push_str(&escape_markup(s));
        }
        out.push_str("</span>");
        last = end;
    }

    if last < len {
        if let Some(s) = name.get(last..) {
            out.push_str(&escape_markup(s));
        }
    }

    out
}

fn escape_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn is_high_contrast() -> bool {
    gtk4::Settings::default()
        .and_then(|s| s.gtk_theme_name())
        .is_some_and(|name| {
            let lower = name.to_lowercase();
            lower.contains("highcontrast") || lower.contains("high-contrast")
        })
}

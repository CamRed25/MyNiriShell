// Polkit agent
use crate::polkit_agent::{PolkitAgent, PolkitRequest};
use crate::polkit_ui::PolkitDialog;
use tokio::runtime::Runtime;
use std::sync::mpsc as std_mpsc;
// use std::rc::Rc; (already imported)
use std::cell::RefCell;
// Unified niri shell — single GTK4 Application running the panel, dock, and launcher.
// Connects to the niri compositor IPC socket for live workspace + window events.

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gtk4::gio::ApplicationFlags;
use gtk4::{glib, prelude::*};

use crate::error::ShellError;
use crate::ipc::IpcEventStream;
use crate::state::ShellState;

const APP_ID: &str = "org.niri.shell";
/// Interval between weather re-fetches.
const WEATHER_INTERVAL: Duration = Duration::from_secs(600);

pub fn run() -> Result<(), ShellError> {
    gtk4::init().map_err(|_| ShellError::GtkInit)?;

    // Channel for polkit requests (tokio -> GTK main thread)
    let (polkit_tx, polkit_rx) = std_mpsc::channel::<PolkitRequest>();
    let polkit_rx = Rc::new(RefCell::new(polkit_rx));

    // Spawn polkit agent in background thread
    std::thread::spawn(move || {
        let rt = Runtime::new().expect("tokio runtime");
        let agent = Arc::new(PolkitAgent::new(polkit_tx));
        rt.block_on(agent.run()).ok();
    });
    // Initialise the sysinfo sampler before the first tick.
    crate::sysinfo::init_sampler();

    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        // NON_UNIQUE: every launch is a fully independent process.
        // Without this, if a prior instance holds the D-Bus name, GTK forwards
        // activate() to it (creating duplicate windows) instead of starting fresh.
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();

    let polkit_rx_c = Rc::clone(&polkit_rx);
    app.connect_activate(move |app| {
        let state = Rc::new(ShellState::new());
        // Drain polkit requests and show dialog (stub)
        let app_weak = app.downgrade();
        glib::timeout_add_local(Duration::from_millis(100), {
            let polkit_rx_c = Rc::clone(&polkit_rx_c);
            move || {
                while let Ok(req) = polkit_rx_c.borrow_mut().try_recv() {
                    if let Some(app) = app_weak.upgrade() {
                        // TODO: Find a suitable parent window
                        let win = app.active_window().unwrap_or_else(|| {
                            gtk4::Window::builder().application(&app).build()
                        });
                        let dialog = PolkitDialog::new(&win, req);
                        dialog.borrow().show();
                    }
                }
                glib::ControlFlow::Continue
            }
        });

        // Wire niri IPC events to shared state; gracefully degrade when unavailable.
        match IpcEventStream::connect() {
            Ok(stream) => {
                let state_ref = Rc::clone(&state);
                stream.attach(move |event| {
                    state_ref.apply_event(event);
                    glib::ControlFlow::Continue
                });
                log::info!("Connected to niri IPC event stream.");
            }
            Err(e) => {
                log::warn!(
                    "Niri IPC unavailable ({}); running without live compositor state.",
                    e
                );
            }
        }

        // Background weather thread — fetches every 10 minutes and hands the
        // result to the GTK main thread via a glib::timeout poll.
        {
            let panel_arc: Arc<Mutex<Option<crate::weather::WeatherSnapshot>>> =
                Arc::new(Mutex::new(None));
            let writer = Arc::clone(&panel_arc);

            std::thread::spawn(move || loop {
                match crate::weather::fetch_weather("auto") {
                    Ok(snap) => {
                        *writer.lock().unwrap() = Some(snap);
                    }
                    Err(e) => log::warn!("weather fetch failed: {e}"),
                }
                std::thread::sleep(WEATHER_INTERVAL);
            });

            let panel_state = Rc::clone(&state.panel);
            glib::timeout_add_local(Duration::from_secs(15), move || {
                if let Some(snap) = panel_arc.lock().unwrap().take() {
                    let _ = panel_state.borrow_mut().update_weather(
                        snap.temperature_c,
                        snap.condition,
                        snap.location,
                    );
                }
                glib::ControlFlow::Continue
            });
        }

        let osd = crate::osd_ui::OsdWindow::new(app);

        // Notification daemon — registers org.freedesktop.Notifications on the session bus.
        let (notif_tx, notif_rx) = std::sync::mpsc::sync_channel::<crate::notification_daemon::DaemonMsg>(64);
        crate::notification_daemon::spawn_daemon(notif_tx);

        // Restore persisted notifications so the notification centre survives a restart.
        for n in crate::notification_daemon::load_persisted_notifications(100) {
            let _ = state.panel.borrow_mut().add_notification(
                n.id, &n.app, &n.summary, &n.body, n.timestamp,
            );
        }

        // Toast overlay and quick-settings window must be created before panel_ui.
        let notif_toasts = crate::notification_ui::NotificationToasts::new(app);
        let qs_win = crate::quick_settings_ui::QuickSettingsWindow::new(app);

        // Drain daemon messages on the GTK main thread every 100 ms.
        {
            use crate::notification_daemon::DaemonMsg;
            let panel_state = Rc::clone(&state.panel);
            let dock_state = Rc::clone(&state.dock);
            let toasts_c = Rc::clone(&notif_toasts);
            let qs_win_c = Rc::clone(&qs_win);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                while let Ok(msg) = notif_rx.try_recv() {
                    match msg {
                        DaemonMsg::Incoming { id, app, summary, body, timeout_ms } => {
                            let ts = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            let _ = panel_state.borrow_mut().add_notification(
                                id, &app, &summary, &body, ts,
                            );
                            crate::notification_daemon::persist_notification(
                                id, &app, &summary, &body, ts,
                            );
                            let counts = panel_state.borrow().notification_counts_by_app();
                            dock_state.borrow_mut().set_notif_counts(counts);
                            if !qs_win_c.is_dnd() {
                                toasts_c.show_toast(&app, &summary, &body, timeout_ms);
                            }
                        }
                        DaemonMsg::Close(id) => {
                            let _ = panel_state.borrow_mut().dismiss_notification(id);
                            let counts = panel_state.borrow().notification_counts_by_app();
                            dock_state.borrow_mut().set_notif_counts(counts);
                        }
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        crate::panel_ui::build_panel_window(app, Rc::clone(&state.panel), Rc::clone(&osd), qs_win);
        crate::dock_ui::build_dock_window(app, Rc::clone(&state.dock));
        crate::launcher_ui::build_launcher_window(app);
        crate::screenshot_ui::build_screenshot_window(app);
    });

    app.run();
    Ok(())
}


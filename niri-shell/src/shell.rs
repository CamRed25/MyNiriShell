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

    // Initialise the sysinfo sampler before the first tick.
    crate::sysinfo::init_sampler();

    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        // NON_UNIQUE: every launch is a fully independent process.
        // Without this, if a prior instance holds the D-Bus name, GTK forwards
        // activate() to it (creating duplicate windows) instead of starting fresh.
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(|app| {
        let state = Rc::new(ShellState::new());

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

        crate::panel_ui::build_panel_window(app, Rc::clone(&state.panel));
        crate::dock_ui::build_dock_window(app, Rc::clone(&state.dock));
        crate::launcher_ui::build_launcher_window(app);
    });

    app.run();
    Ok(())
}


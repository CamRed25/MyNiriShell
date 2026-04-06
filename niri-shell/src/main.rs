// Entry point for the niri unified shell.

mod dock_backend;
mod dock_ui;
mod error;
mod ipc;
mod launcher_backend;
mod launcher_ui;
mod media;
mod panel_backend;
mod panel_ui;
mod shell;
mod state;
mod sysinfo;
mod weather;

fn main() {
    env_logger::init();

    // Replace any prior running instance before creating new windows.
    // niri auto-restarts spawn-at-startup processes, so without this we'd
    // briefly have two shells running simultaneously.
    kill_prior_instances();

    if let Err(e) = shell::run() {
        log::error!("niri-shell: {e}");
        std::process::exit(1);
    }
}

/// Sends SIGTERM to any other niri-shell processes with the same binary path.
fn kill_prior_instances() {
    let Ok(self_path) = std::fs::read_link("/proc/self/exe") else {
        return;
    };
    let Ok(self_pid) = std::fs::read_to_string("/proc/self/stat") else {
        return;
    };
    let self_pid: u32 = self_pid
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let Ok(entries) = std::fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Ok(pid): Result<u32, _> = name.to_string_lossy().parse() else {
            continue;
        };
        if pid == self_pid {
            continue;
        }
        if let Ok(path) = std::fs::read_link(format!("/proc/{pid}/exe")) {
            if path == self_path {
                log::info!("replacing prior niri-shell instance PID {pid}");
                // SAFETY: kill(2) is safe with a valid signal number.
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
            }
        }
    }
    // Give the prior instance time to clean up its GTK/layer-shell resources.
    std::thread::sleep(std::time::Duration::from_millis(300));
}


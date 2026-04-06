// Entry point for the Niri shell backend. Initializes all core subsystems and runs the event loop.

mod error;
mod shell;

fn main() {
    if let Err(e) = shell::run() {
        eprintln!("Shell failed to start: {e}");
        std::process::exit(1);
    }
}


mod dock_backend;
mod dock_ui;

fn main() {
    env_logger::init();
    if let Err(e) = dock_ui::run() {
        log::error!("Dock failed to start: {e}");
        std::process::exit(1);
    }
}

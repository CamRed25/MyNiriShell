mod panel_backend;
mod panel_ui;

fn main() {
    if let Err(e) = panel_ui::run() {
        log::erroror!("Panel failed to start: {e}");
        std::process::exit(1);
    }
}

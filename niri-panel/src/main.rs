mod panel_backend;
mod panel_ui;

fn main() {
    if let Err(e) = panel_ui::run() {
        eprintln!("Panel failed to start: {e}");
        std::process::exit(1);
    }
}

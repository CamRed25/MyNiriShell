// On-screen display overlay — volume / brightness feedback pill.
// Appears centred on screen, auto-dismisses after 1.5 s.
// No business logic.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::{glib, prelude::*, Application, ApplicationWindow, Box as GtkBox, Label, Orientation};
use gtk4_layer_shell::{Layer, LayerShell};

const CSS: &str = r#"
.osd-pill {
    background: rgba(15, 15, 25, 0.90);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 24px;
    padding: 12px 28px;
}

.osd-icon {
    font-size: 20px;
}

.osd-value {
    font-size: 17px;
    font-weight: bold;
    color: #c0caf5;
    font-family: "Inter", "Noto Sans", sans-serif;
    margin-left: 8px;
}
"#;

pub struct OsdWindow {
    window: ApplicationWindow,
    icon_lbl: Label,
    value_lbl: Label,
    /// Generation counter — incremented on every `show()`. The auto-hide timer
    /// only fires if the generation hasn't changed (i.e. no newer show() arrived).
    generation: Rc<Cell<u32>>,
}

impl OsdWindow {
    pub fn new(app: &Application) -> Rc<Self> {
        if let Some(display) = gtk4::gdk::Display::default() {
            let provider = gtk4::CssProvider::new();
            provider.load_from_string(CSS);
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let window = ApplicationWindow::builder()
            .application(app)
            .title("niri-osd")
            .decorated(false)
            .resizable(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        // No edge anchoring → centred on screen; no exclusive zone.

        let pill = GtkBox::new(Orientation::Horizontal, 0);
        pill.add_css_class("osd-pill");
        pill.set_halign(gtk4::Align::Center);
        pill.set_valign(gtk4::Align::Center);

        let icon_lbl = Label::new(Some("🔊"));
        icon_lbl.add_css_class("osd-icon");

        let value_lbl = Label::new(Some("0%"));
        value_lbl.add_css_class("osd-value");

        pill.append(&icon_lbl);
        pill.append(&value_lbl);
        window.set_child(Some(&pill));
        window.set_visible(false);

        Rc::new(Self {
            window,
            icon_lbl,
            value_lbl,
            generation: Rc::new(Cell::new(0)),
        })
    }

    /// Show the OSD displaying `icon` and `value`%; auto-hides after 1.5 s.
    pub fn show(&self, icon: &str, value: u8) {
        self.icon_lbl.set_text(icon);
        self.value_lbl.set_text(&format!("{}%", value));
        self.window.set_visible(true);
        self.window.present();

        let gen = self.generation.get().wrapping_add(1);
        self.generation.set(gen);

        let win_weak = self.window.downgrade();
        let gen_ref = Rc::clone(&self.generation);
        glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
            if gen_ref.get() == gen {
                if let Some(w) = win_weak.upgrade() {
                    w.set_visible(false);
                }
            }
        });
    }
}

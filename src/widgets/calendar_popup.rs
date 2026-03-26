use crate::shell::ShellPopup;
use crate::widgets::base::PopupBase;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, LayerShell};
use std::rc::Rc;

pub struct CalendarPopup {
    base: PopupBase,
}

impl ShellPopup for CalendarPopup {
    fn id(&self) -> &str {
        "calendar"
    }

    fn is_open(&self) -> bool {
        self.base.is_open.get()
    }

    fn toggle(&self) {
        if self.is_open() {
            self.close();
        } else {
            self.base.open();
        }
    }

    fn close(&self) {
        self.base
            .window
            .set_keyboard_mode(KeyboardMode::OnDemand);
        self.base.close();
    }
}

impl CalendarPopup {
    pub fn new(
        app: &libadwaita::Application,
        on_state_change: impl Fn() + 'static,
    ) -> Self {
        let base = PopupBase::new(app, "AXIS Calendar", false);

        // Center on screen (remove left/right anchors)
        base.window.set_anchor(Edge::Left, false);
        base.window.set_anchor(Edge::Right, false);

        let on_change = Rc::new(on_state_change);
        base.window
            .connect_visible_notify(move |_| on_change());

        let calendar = gtk4::Calendar::new();

        let wrapper = gtk4::Box::builder()
            .css_classes(vec!["calendar-wrapper".to_string()])
            .build();
        wrapper.append(&calendar);
        base.set_content(&wrapper);

        // Escape closes
        let base_close = base.clone();
        let key = gtk4::EventControllerKey::new();
        key.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                base_close.close();
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        base.window.add_controller(key);

        Self { base }
    }
}

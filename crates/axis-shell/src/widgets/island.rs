use libadwaita::prelude::*;

#[derive(Clone)]
pub struct Island {
    pub container: gtk4::Box,
}

impl Island {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        container.add_css_class("island");
        Self { container }
    }

    pub fn on_clicked<F: Fn() + 'static>(&self, f: F) {
        let gesture = gtk4::GestureClick::new();
        gesture.connect_released(move |_, _, _, _| {
            f();
        });
        self.container.add_controller(gesture);
        self.container.set_cursor_from_name(Some("pointer"));
    }
}

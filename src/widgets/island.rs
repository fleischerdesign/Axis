use gtk4::prelude::*;

pub struct Island {
    pub container: gtk4::Box,
}

impl Island {
    pub fn new(spacing: i32) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, spacing);
        container.add_css_class("island");
        Self { container }
    }

    pub fn append<P: IsA<gtk4::Widget>>(&self, child: &P) {
        self.container.append(child);
    }
}

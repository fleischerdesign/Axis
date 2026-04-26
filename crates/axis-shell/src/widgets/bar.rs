use libadwaita::prelude::*;

pub struct Bar {
    pub container: gtk4::Box,
    center_box: gtk4::CenterBox,
}

impl Bar {
    pub fn new() -> Self {
        let center_box = gtk4::CenterBox::new();
        center_box.set_hexpand(true);
        center_box.set_valign(gtk4::Align::Start);
        center_box.set_height_request(44);
        center_box.add_css_class("bar-container");

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.set_height_request(54);
        container.set_vexpand(true);
        container.add_css_class("bar-main-widget");
        container.append(&center_box);

        Self { container, center_box }
    }

    pub fn set_start_widget<P: IsA<gtk4::Widget>>(&self, widget: Option<&P>) {
        self.center_box.set_start_widget(widget);
    }

    pub fn set_center_widget<P: IsA<gtk4::Widget>>(&self, widget: Option<&P>) {
        self.center_box.set_center_widget(widget);
    }

    pub fn set_end_widget<P: IsA<gtk4::Widget>>(&self, widget: Option<&P>) {
        self.center_box.set_end_widget(widget);
    }
}

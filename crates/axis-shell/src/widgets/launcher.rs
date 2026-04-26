use libadwaita::prelude::*;

pub struct LauncherWidget {
    pub container: gtk4::Box,
}

impl LauncherWidget {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.add_css_class("launcher-widget");

        let icon = gtk4::Image::from_icon_name("view-app-grid-symbolic");
        icon.set_pixel_size(20);
        icon.add_css_class("status-icon");
        container.append(&icon);

        Self { container }
    }
}

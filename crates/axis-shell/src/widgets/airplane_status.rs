use libadwaita::prelude::*;
use axis_presentation::View;
use axis_domain::models::airplane::AirplaneStatus;

#[derive(Clone)]
pub struct AirplaneStatusWidget {
    pub container: gtk4::Box,
    icon: gtk4::Image,
}

impl AirplaneStatusWidget {
    pub fn new() -> Self {
        let icon = gtk4::Image::new();
        icon.set_pixel_size(20);
        icon.add_css_class("status-icon");

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.append(&icon);
        container.set_visible(false);

        Self { container, icon }
    }
}

impl View<AirplaneStatus> for AirplaneStatusWidget {
    fn render(&self, status: &AirplaneStatus) {
        self.container.set_visible(status.enabled && status.available);
        if status.enabled && status.available {
            self.icon.set_icon_name(Some("airplane-mode-symbolic"));
        }
    }
}

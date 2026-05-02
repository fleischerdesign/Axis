use libadwaita::prelude::*;
use axis_presentation::View;
use axis_domain::models::idle_inhibit::IdleInhibitStatus;

#[derive(Clone)]
pub struct IdleInhibitStatusWidget {
    pub container: gtk4::Box,
    icon: gtk4::Image,
}

impl IdleInhibitStatusWidget {
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

impl View<IdleInhibitStatus> for IdleInhibitStatusWidget {
    fn render(&self, status: &IdleInhibitStatus) {
        self.container.set_visible(status.inhibited);
        if status.inhibited {
            self.icon.set_icon_name(Some("changes-prevent-symbolic"));
        }
    }
}

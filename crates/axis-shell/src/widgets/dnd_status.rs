use libadwaita::prelude::*;
use axis_presentation::View;
use axis_domain::models::dnd::DndStatus;

#[derive(Clone)]
pub struct DndStatusWidget {
    pub container: gtk4::Box,
    icon: gtk4::Image,
}

impl DndStatusWidget {
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

impl View<DndStatus> for DndStatusWidget {
    fn render(&self, status: &DndStatus) {
        self.container.set_visible(status.enabled);
        if status.enabled {
            self.icon.set_icon_name(Some("notifications-disabled-symbolic"));
        }
    }
}

use axis_domain::models::continuity::ContinuityStatus;
use axis_presentation::View;
use libadwaita::prelude::*;

#[derive(Clone)]
pub struct ContinuityStatusWidget {
    pub container: gtk4::Box,
    icon: gtk4::Image,
}

impl Default for ContinuityStatusWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ContinuityStatusWidget {
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

impl View<ContinuityStatus> for ContinuityStatusWidget {
    fn render(&self, status: &ContinuityStatus) {
        self.container.set_visible(status.enabled);
        if status.enabled {
            self.icon.set_icon_name(Some("input-mouse-symbolic"));
        }
    }
}

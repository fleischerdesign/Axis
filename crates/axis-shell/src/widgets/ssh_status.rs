use gtk4::prelude::*;
use axis_domain::models::ssh::SshStatus;
use axis_presentation::View;

#[derive(Clone)]
pub struct SshStatusWidget {
    pub container: gtk4::Box,
    badge: gtk4::Label,
}

impl SshStatusWidget {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
        container.add_css_class("island-widget");
        container.set_visible(false);

        let icon = gtk4::Image::from_icon_name("network-server-symbolic");
        icon.set_pixel_size(18);

        let badge = gtk4::Label::new(None);

        container.append(&icon);
        container.append(&badge);

        Self { container, badge }
    }
}

impl View<SshStatus> for SshStatusWidget {
    fn render(&self, status: &SshStatus) {
        let visible = status.active_count > 0;
        self.container.set_visible(visible);
        if visible {
            self.badge.set_text(&status.active_count.to_string());
        }
    }
}

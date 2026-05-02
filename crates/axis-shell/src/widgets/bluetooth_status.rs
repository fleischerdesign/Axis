use libadwaita::prelude::*;
use axis_presentation::View;
use axis_domain::models::bluetooth::BluetoothStatus;

#[derive(Clone)]
pub struct BluetoothStatusWidget {
    pub container: gtk4::Box,
    icon: gtk4::Image,
}

impl BluetoothStatusWidget {
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

impl View<BluetoothStatus> for BluetoothStatusWidget {
    fn render(&self, status: &BluetoothStatus) {
        self.container.set_visible(status.powered);
        if status.powered {
            self.icon.set_icon_name(Some("bluetooth-active-symbolic"));
        }
    }
}

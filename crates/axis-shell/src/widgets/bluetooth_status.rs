use axis_domain::models::bluetooth::BluetoothStatus;
use axis_presentation::View;
use libadwaita::prelude::*;

#[derive(Clone)]
pub struct BluetoothStatusWidget {
    pub container: gtk4::Box,
    icon: gtk4::Image,
}

impl Default for BluetoothStatusWidget {
    fn default() -> Self {
        Self::new()
    }
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
            let has_connected = status.devices.iter().any(|d| d.connected);
            let icon_name = if has_connected {
                "bluetooth-active-symbolic"
            } else {
                "bluetooth-disconnected-symbolic"
            };
            self.icon.set_icon_name(Some(icon_name));
        }
    }
}

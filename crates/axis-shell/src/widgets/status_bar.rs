use libadwaita::prelude::*;
use axis_presentation::View;
use crate::presentation::battery::battery_icon;
use axis_domain::models::power::PowerStatus;

#[derive(Clone)]
pub struct StatusBar {
    pub container: gtk4::Box,
    icon: gtk4::Image,
    label: gtk4::Label,
}

impl StatusBar {
    pub fn new(show_labels: bool) -> Self {
        let icon = gtk4::Image::new();
        icon.set_pixel_size(20);
        icon.add_css_class("status-icon");

        let label = gtk4::Label::new(None);
        label.add_css_class("status-text");
        label.set_visible(show_labels);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.append(&icon);
        container.append(&label);
        container.add_css_class("status-bar");

        Self { container, icon, label }
    }
}

impl View<PowerStatus> for StatusBar {
    fn render(&self, status: &PowerStatus) {
        self.container.set_visible(status.has_battery);
        if status.has_battery {
            let icon_name = battery_icon(status.battery_percentage, status.is_charging).to_string();
            self.icon.set_icon_name(Some(&icon_name));
            if self.label.is_visible() {
                self.label.set_label(&format!("{:.0}%", status.battery_percentage));
            }
            if status.is_charging {
                self.icon.add_css_class("charging");
            } else {
                self.icon.remove_css_class("charging");
            }
        }
    }
}

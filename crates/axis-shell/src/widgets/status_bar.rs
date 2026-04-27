use libadwaita::prelude::*;
use axis_presentation::View;
use crate::presentation::battery::{BatteryView, battery_icon};
use axis_domain::models::power::PowerStatus;

#[derive(Clone)]
pub struct StatusBar {
    pub container: gtk4::Box,
    icon: gtk4::Image,
    label: gtk4::Label,
}

impl StatusBar {
    pub fn new() -> Self {
        let icon = gtk4::Image::new();
        icon.set_pixel_size(20);
        icon.add_css_class("status-icon");

        let label = gtk4::Label::new(None);
        label.add_css_class("status-text");

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.append(&icon);
        container.append(&label);
        container.add_css_class("status-bar");

        Self { container, icon, label }
    }
}

impl View<PowerStatus> for StatusBar {
    fn render(&self, status: &PowerStatus) {
        let icon_name = battery_icon(status.battery_percentage, status.is_charging).to_string();
        let percentage_text = format!("{:.0}%", status.battery_percentage);
        self.icon.set_icon_name(Some(&icon_name));
        self.label.set_label(&percentage_text);
        if status.is_charging {
            self.icon.add_css_class("charging");
        } else {
            self.icon.remove_css_class("charging");
        }
    }
}

impl BatteryView for StatusBar {}

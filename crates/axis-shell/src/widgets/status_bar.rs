use libadwaita::prelude::*;
use gtk4::glib;
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
        label.set_visible(false);

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
        let is_charging = status.is_charging;
        let icon = self.icon.clone();
        let label = self.label.clone();
        glib::idle_add_local(move || {
            icon.set_icon_name(Some(&icon_name));
            label.set_label(&percentage_text);
            if is_charging {
                icon.add_css_class("charging");
            } else {
                icon.remove_css_class("charging");
            }
            glib::ControlFlow::Break
        });
    }
}

impl BatteryView for StatusBar {}

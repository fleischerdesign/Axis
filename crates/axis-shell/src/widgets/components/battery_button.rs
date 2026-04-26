use libadwaita::prelude::*;
use gtk4::glib;
use axis_domain::models::power::PowerStatus;
use crate::presentation::battery::{BatteryView, battery_icon};
use axis_presentation::View;

#[derive(Clone, Debug)]
pub struct BatteryButton {
    pub container: gtk4::Button,
    icon: gtk4::Image,
    label: gtk4::Label,
}

impl BatteryButton {
    pub fn new() -> Self {
        let icon = gtk4::Image::new();
        icon.set_pixel_size(20);

        let label = gtk4::Label::new(None);

        let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        content.append(&icon);
        content.append(&label);

        let container = gtk4::Button::new();
        container.set_child(Some(&content));
        container.add_css_class("qs-battery-btn");
        container.set_visible(false);

        Self { container, icon, label }
    }
}

impl View<PowerStatus> for BatteryButton {
    fn render(&self, status: &PowerStatus) {
        let visible = status.has_battery;
        let icon_name = battery_icon(status.battery_percentage, status.is_charging).to_string();
        let pct_text = format!("{:.0}%", status.battery_percentage);
        let container = self.container.clone();
        let icon = self.icon.clone();
        let label = self.label.clone();
        glib::idle_add_local(move || {
            container.set_visible(visible);
            if visible {
                icon.set_icon_name(Some(&icon_name));
                label.set_label(&pct_text);
            }
            glib::ControlFlow::Break
        });
    }
}

impl BatteryView for BatteryButton {}

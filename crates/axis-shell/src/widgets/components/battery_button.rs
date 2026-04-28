use libadwaita::prelude::*;
use axis_domain::models::power::PowerStatus;
use crate::presentation::battery::battery_icon;
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
        self.container.set_visible(status.has_battery);
        if status.has_battery {
            let icon_name = battery_icon(status.battery_percentage, status.is_charging).to_string();
            let pct_text = format!("{:.0}%", status.battery_percentage);
            self.icon.set_icon_name(Some(&icon_name));
            self.label.set_label(&pct_text);
        }
    }
}

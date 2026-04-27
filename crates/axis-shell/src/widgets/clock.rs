use libadwaita::prelude::*;
use axis_presentation::View;
use axis_domain::models::clock::TimeStatus;

#[derive(Clone)]
pub struct ClockWidget {
    pub container: gtk4::Box,
    label: gtk4::Label,
}

impl ClockWidget {
    pub fn new() -> Self {
        let label = gtk4::Label::new(None);
        label.add_css_class("clock-label");

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.append(&label);

        Self { container, label }
    }
}

impl View<TimeStatus> for ClockWidget {
    fn render(&self, status: &TimeStatus) {
        let time_str = status.current_time.format("%H:%M:%S").to_string();
        self.label.set_label(&time_str);
    }
}

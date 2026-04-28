use libadwaita::prelude::*;
use axis_presentation::View;
use crate::presentation::audio::audio_icon;
use axis_domain::models::audio::AudioStatus;

#[derive(Clone)]
pub struct AudioWidget {
    pub container: gtk4::Box,
    icon: gtk4::Image,
    label: gtk4::Label,
}

impl AudioWidget {
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
        container.add_css_class("audio-widget");

        Self { container, icon, label }
    }
}

impl View<AudioStatus> for AudioWidget {
    fn render(&self, status: &AudioStatus) {
        let icon_name = audio_icon(status).to_string();
        self.icon.set_icon_name(Some(&icon_name));
        if self.label.is_visible() {
            self.label.set_label(&format!("{:.0}%", status.volume * 100.0));
        }
    }
}


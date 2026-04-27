use libadwaita::prelude::*;
use axis_presentation::View;
use crate::presentation::audio::{AudioView, audio_icon};
use axis_domain::models::audio::AudioStatus;

#[derive(Clone)]
pub struct AudioWidget {
    pub container: gtk4::Box,
    icon: gtk4::Image,
}

impl AudioWidget {
    pub fn new() -> Self {
        let icon = gtk4::Image::new();
        icon.set_pixel_size(20);
        icon.add_css_class("status-icon");

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.append(&icon);
        container.add_css_class("audio-widget");

        Self { container, icon }
    }
}

impl View<AudioStatus> for AudioWidget {
    fn render(&self, status: &AudioStatus) {
        let icon_name = audio_icon(status).to_string();
        self.icon.set_icon_name(Some(&icon_name));
    }
}

impl AudioView for AudioWidget {
    fn on_volume_changed(&self, _f: Box<dyn Fn(f64) + 'static>) {}
    fn on_set_default_sink(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_default_source(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_sink_input_volume(&self, _f: Box<dyn Fn(u32, f64) + 'static>) {}
}

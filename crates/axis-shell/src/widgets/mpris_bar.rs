use libadwaita::prelude::*;
use axis_presentation::View;
use axis_domain::models::mpris::MprisStatus;

#[derive(Clone)]
pub struct MprisBarWidget {
    pub container: gtk4::Box,
    label: gtk4::Label,
}

impl MprisBarWidget {
    pub fn new() -> Self {
        let icon = gtk4::Image::from_icon_name("audio-x-generic-symbolic");
        icon.set_pixel_size(16);
        icon.add_css_class("status-icon");

        let label = gtk4::Label::new(None);
        label.add_css_class("mpris-label");
        label.set_max_width_chars(30);
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        container.append(&icon);
        container.append(&label);
        container.add_css_class("mpris-bar");
        container.set_visible(false);

        Self { container, label }
    }
}

impl View<MprisStatus> for MprisBarWidget {
    fn render(&self, status: &MprisStatus) {
        match status.active_player() {
            Some(player) => {
                self.container.set_visible(true);
                let text = if player.artist.is_empty() {
                    player.title.clone()
                } else {
                    format!("{} \u{2014} {}", player.artist, player.title)
                };
                self.label.set_label(&text);
            }
            None => {
                self.container.set_visible(false);
            }
        }
    }
}

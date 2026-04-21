use gtk4::prelude::*;

#[derive(Clone)]
pub struct BaseIcon {
    pub image: gtk4::Image,
}

impl BaseIcon {
    pub fn new(icon_name: &str) -> Self {
        let image = gtk4::Image::from_icon_name(icon_name);
        Self { image }
    }

    pub fn set(&self, icon_name: &str) {
        self.image.set_icon_name(Some(icon_name));
    }

    pub fn set_visible(&self, visible: bool) {
        self.image.set_visible(visible);
    }
}

use crate::widgets::Island;
use gtk4::prelude::*;

pub struct BarLauncher {
    pub container: gtk4::Box,
}

impl BarLauncher {
    pub fn new() -> Self {
        let island = Island::new(0);
        island.container.set_cursor_from_name(Some("pointer"));
        island.append(&gtk4::Image::from_icon_name("view-app-grid-symbolic"));

        Self {
            container: island.container,
        }
    }
}

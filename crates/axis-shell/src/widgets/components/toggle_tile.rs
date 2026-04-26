use libadwaita::prelude::*;
use gtk4::glib;
use axis_presentation::View;

#[derive(Clone)]
pub struct ToggleTile {
    pub container: gtk4::Box,
    main_btn: gtk4::Button,
    arrow_btn: gtk4::Button,
    icon: gtk4::Image,
    label: gtk4::Label,
}

impl ToggleTile {
    pub fn new(label_text: &str, icon_name: &str, has_arrow: bool) -> Self {
        let main_btn = gtk4::Button::new();
        main_btn.add_css_class("tile-main");
        main_btn.set_hexpand(true);
        main_btn.add_css_class("sole");

        let icon = gtk4::Image::new();
        icon.set_pixel_size(18);

        let label = gtk4::Label::new(None);
        label.add_css_class("tile-label");

        let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        content.append(&icon);
        content.append(&label);
        main_btn.set_child(Some(&content));

        let arrow_btn = gtk4::Button::new();
        arrow_btn.set_icon_name("go-next-symbolic");
        arrow_btn.add_css_class("tile-arrow");
        arrow_btn.set_visible(false);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.add_css_class("tile");
        container.append(&main_btn);
        container.append(&arrow_btn);

        let tile = Self { container, main_btn, arrow_btn, icon, label };
        tile.set_label(label_text);
        tile.set_icon(icon_name);
        tile.set_show_arrow(has_arrow);
        tile
    }

    pub fn set_label(&self, label: &str) {
        self.label.set_label(label);
    }

    pub fn set_icon(&self, icon_name: &str) {
        self.icon.set_icon_name(Some(icon_name));
    }

    pub fn set_active(&self, active: bool) {
        if active {
            self.container.add_css_class("active");
        } else {
            self.container.remove_css_class("active");
        }
    }

    pub fn set_show_arrow(&self, show: bool) {
        self.arrow_btn.set_visible(show);
        if show {
            self.main_btn.remove_css_class("sole");
        } else {
            self.main_btn.add_css_class("sole");
        }
    }

    pub fn on_clicked<F: Fn() + 'static>(&self, f: F) {
        self.main_btn.connect_clicked(move |_| {
            f();
        });
    }

    pub fn on_arrow_clicked<F: Fn() + 'static>(&self, f: F) {
        self.arrow_btn.connect_clicked(move |_| {
            f();
        });
    }
}

impl View<bool> for ToggleTile {
    fn render(&self, status: &bool) {
        self.set_active(*status);
    }
}

impl crate::presentation::toggle::ToggleView for ToggleTile {
    fn set_icon(&self, icon_name: &str) {
        self.set_icon(icon_name);
    }

    fn set_label(&self, label: &str) {
        self.set_label(label);
    }

    fn on_toggled(&self, f: Box<dyn Fn(bool) + 'static>) {
        let tile = self.clone();
        self.on_clicked(move || {
            let next_state = !tile.container.has_css_class("active");
            f(next_state);
        });
    }
}

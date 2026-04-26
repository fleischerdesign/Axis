use libadwaita::prelude::*;
use gtk4::glib;

#[derive(Clone, Debug)]
pub struct QuickSlider {
    pub container: gtk4::Box,
    overlay: gtk4::Overlay,
    icon: gtk4::Image,
    scale: gtk4::Scale,
    arrow_btn: gtk4::Button,
}

impl QuickSlider {
    pub fn new(icon_name: &str) -> Self {
        let icon = gtk4::Image::new();
        icon.set_pixel_size(22);
        icon.set_halign(gtk4::Align::Start);
        icon.set_valign(gtk4::Align::Center);
        icon.set_margin_start(22);
        icon.set_can_target(false);
        icon.add_css_class("slider-icon-overlay");

        let scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        scale.set_draw_value(false);
        scale.set_hexpand(true);
        scale.set_valign(gtk4::Align::Center);

        let arrow_btn = gtk4::Button::new();
        arrow_btn.set_icon_name("go-next-symbolic");
        arrow_btn.add_css_class("tile-arrow");
        arrow_btn.set_visible(false);

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&scale));
        overlay.add_overlay(&icon);
        overlay.set_hexpand(true);

        let arrow_c = arrow_btn.clone();
        let scale_c = scale.clone();
        scale.connect_value_changed(move |s| {
            let is_full = s.value() >= s.adjustment().upper() - 0.01;
            if is_full {
                arrow_c.add_css_class("max");
                scale_c.remove_css_class("highlight-partial");
            } else {
                arrow_c.remove_css_class("max");
                scale_c.add_css_class("highlight-partial");
            }
        });

        scale.add_css_class("highlight-partial");

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.add_css_class("slider-row");
        container.add_css_class("volume-slider");
        container.append(&overlay);
        container.append(&arrow_btn);

        let slider = Self { container, overlay, icon, scale, arrow_btn };
        slider.set_icon(icon_name);
        slider
    }

    pub fn set_icon(&self, icon_name: &str) {
        self.icon.set_icon_name(Some(icon_name));
    }

    pub fn scale(&self) -> gtk4::Scale {
        self.scale.clone()
    }

    pub fn set_show_arrow(&self, show: bool) {
        self.arrow_btn.set_visible(show);
        if show {
            self.scale.add_css_class("with-arrow");
        } else {
            self.scale.remove_css_class("with-arrow");
        }
    }

    pub fn on_arrow_clicked<F: Fn() + 'static>(&self, f: F) {
        self.arrow_btn.connect_clicked(move |_| {
            f();
        });
    }
}

use gtk4::prelude::*;

/// A horizontal slider with a leading icon overlay.
/// CSS class is not set by default — add it via overlay.add_css_class().
#[derive(Clone)]
pub struct IconSlider {
    pub overlay: gtk4::Overlay,
    pub slider: gtk4::Scale,
    pub icon: gtk4::Image,
}

impl IconSlider {
    pub fn new(icon_name: &str, min: f64, max: f64, step: f64) -> Self {
        let overlay = gtk4::Overlay::new();
        overlay.set_hexpand(true);

        let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, min, max, step);
        slider.set_hexpand(true);

        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(22);
        icon.set_margin_start(22);
        icon.set_halign(gtk4::Align::Start);
        icon.set_valign(gtk4::Align::Center);
        icon.set_can_target(false);

        overlay.set_child(Some(&slider));
        overlay.add_overlay(&icon);

        Self {
            overlay,
            slider,
            icon,
        }
    }

    pub fn set_value(&self, val: f64) {
        self.slider.set_value(val);
    }

    pub fn connect_value_changed<F: Fn(f64) + 'static>(&self, f: F) {
        self.slider.connect_value_changed(move |s| f(s.value()));
    }

    pub fn set_icon(&self, icon_name: &str) {
        self.icon.set_icon_name(Some(icon_name));
    }
}

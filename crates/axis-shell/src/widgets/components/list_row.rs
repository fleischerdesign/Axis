use libadwaita::prelude::*;

#[derive(Clone)]
pub struct ListRow {
    pub container: gtk4::Box,
    icon: gtk4::Image,
    title: gtk4::Label,
    subtitle: gtk4::Label,
    checkmark: gtk4::Image,
    trailing: gtk4::Box,
}

impl ListRow {
    pub fn new(title: &str, icon_name: &str) -> Self {
        let icon = gtk4::Image::new();
        icon.set_pixel_size(18);

        let title_label = gtk4::Label::builder()
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        let subtitle = gtk4::Label::builder()
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(45)
            .css_classes(vec!["list-sublabel"])
            .visible(false)
            .build();

        let checkmark = gtk4::Image::from_icon_name("object-select-symbolic");
        checkmark.set_halign(gtk4::Align::End);
        checkmark.set_valign(gtk4::Align::Center);
        checkmark.set_visible(false);

        let trailing = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        trailing.set_visible(false);

        let label_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        label_box.set_hexpand(true);
        label_box.append(&title_label);
        label_box.append(&subtitle);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.add_css_class("list-row");
        container.append(&icon);
        container.append(&label_box);
        container.append(&trailing);
        container.append(&checkmark);

        let row = Self { container, icon, title: title_label, subtitle, checkmark, trailing };
        row.set_title(title);
        row.set_icon(icon_name);
        row
    }

    pub fn set_title(&self, title: &str) {
        self.title.set_label(title);
    }

    pub fn set_icon(&self, icon: &str) {
        self.icon.set_icon_name(Some(icon));
    }

    pub fn set_subtitle(&self, text: Option<&str>) {
        if let Some(t) = text {
            self.subtitle.set_label(t);
            self.subtitle.set_visible(true);
        } else {
            self.subtitle.set_visible(false);
        }
    }

    pub fn set_active(&self, active: bool) {
        if active {
            self.container.add_css_class("active");
        } else {
            self.container.remove_css_class("active");
        }
    }

    pub fn set_show_checkmark(&self, show: bool) {
        self.checkmark.set_visible(show);
    }

    pub fn set_trailing(&self, widget: Option<&gtk4::Widget>) {
        if let Some(w) = widget {
            self.trailing.append(w);
            self.trailing.set_visible(true);
        } else {
            while let Some(child) = self.trailing.first_child() {
                self.trailing.remove(&child);
            }
            self.trailing.set_visible(false);
        }
    }
}

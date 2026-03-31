use gtk4::prelude::*;

pub struct ListRow {
    pub container: gtk4::Box,
    pub button: gtk4::Button,
    icon_img: gtk4::Image,
    label: gtk4::Label,
    sublabel: gtk4::Label,
    check_img: gtk4::Image,
}

impl ListRow {
    pub fn new(
        label: &str,
        icon: &str,
        active: bool,
        sublabel: Option<&str>,
        show_check: bool,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let button = gtk4::Button::builder()
            .css_classes(vec!["list-row".to_string()])
            .focusable(false)
            .build();

        if active {
            button.add_css_class("active");
        }

        let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(8);
        content.set_margin_bottom(8);

        let icon_img = gtk4::Image::from_icon_name(icon);
        icon_img.set_pixel_size(18);
        content.append(&icon_img);

        let label_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        label_box.set_hexpand(true);

        let label_widget = gtk4::Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        label_box.append(&label_widget);

        let sublabel_widget = {
            let sl = gtk4::Label::builder()
                .label(sublabel.unwrap_or(""))
                .halign(gtk4::Align::Start)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .max_width_chars(35)
                .css_classes(vec!["list-sublabel".to_string()])
                .visible(sublabel.is_some())
                .build();
            label_box.append(&sl);
            sl
        };

        content.append(&label_box);

        let check_img = {
            let check = gtk4::Image::from_icon_name("object-select-symbolic");
            check.set_halign(gtk4::Align::End);
            check.set_visible(active && show_check);
            content.append(&check);
            check
        };

        button.set_child(Some(&content));
        container.append(&button);

        Self {
            container,
            button,
            icon_img,
            label: label_widget,
            sublabel: sublabel_widget,
            check_img,
        }
    }

    pub fn update(
        &self,
        label: &str,
        icon: &str,
        active: bool,
        sublabel: Option<&str>,
        show_check: bool,
    ) {
        self.label.set_label(label);
        self.icon_img.set_icon_name(Some(icon));

        if active {
            self.button.add_css_class("active");
        } else {
            self.button.remove_css_class("active");
        }

        if let Some(text) = sublabel {
            self.sublabel.set_label(text);
            self.sublabel.set_visible(true);
        } else {
            self.sublabel.set_visible(false);
        }

        self.check_img.set_visible(active && show_check);
    }
}

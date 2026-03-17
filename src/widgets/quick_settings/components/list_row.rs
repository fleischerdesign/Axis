use gtk4::prelude::*;

pub struct QsListRow {
    pub container: gtk4::Box,
    pub button: gtk4::Button,
}

impl QsListRow {
    pub fn new(label: &str, icon: &str, active: bool, sublabel: Option<&str>, show_check: bool) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let button = gtk4::Button::builder()
            .css_classes(vec!["qs-list-row".to_string()])
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
        label_box.append(
            &gtk4::Label::builder()
                .label(label)
                .halign(gtk4::Align::Start)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build(),
        );

        if let Some(sub) = sublabel {
            label_box.append(
                &gtk4::Label::builder()
                    .label(sub)
                    .halign(gtk4::Align::Start)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .max_width_chars(35)
                    .css_classes(vec!["qs-list-sublabel".to_string()])
                    .build(),
            );
        }
        content.append(&label_box);

        // Haken nur anzeigen, wenn explizit gewünscht (WLAN/BT Status)
        if active && show_check {
            let check = gtk4::Image::from_icon_name("object-select-symbolic");
            check.set_halign(gtk4::Align::End);
            content.append(&check);
        }

        button.set_child(Some(&content));
        container.append(&button);

        Self { container, button }
    }
}

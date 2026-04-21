use gtk4::prelude::*;

pub struct PopupHeader {
    pub container: gtk4::Box,
    back_btn: gtk4::Button,
}

impl PopupHeader {
    pub fn new(title: &str) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        container.add_css_class("popup-header");
        container.set_margin_start(4);
        container.set_margin_end(4);
        container.set_margin_top(4);
        container.set_margin_bottom(4);

        let back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["circular".to_string(), "popup-back-btn".to_string()])
            .build();

        let title_label = gtk4::Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .css_classes(vec!["title-4".to_string()])
            .build();

        container.append(&back_btn);
        container.append(&title_label);

        Self { container, back_btn }
    }

    pub fn with_spinner(title: &str, spinner: &gtk4::Spinner) -> Self {
        let header = Self::new(title);
        header.container.append(spinner);
        header
    }

    pub fn connect_back<F: Fn() + 'static>(&self, f: F) {
        self.back_btn.connect_clicked(move |_| {
            f();
        });
    }
}

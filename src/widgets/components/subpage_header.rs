use gtk4::prelude::*;

pub struct SubPageHeader {
    pub container: gtk4::Box,
    back_btn: gtk4::Button,
}

impl SubPageHeader {
    pub fn new(title: &str) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        let back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["subpage-back-btn".to_string()])
            .build();

        let title_label = gtk4::Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(vec!["subpage-title".to_string()])
            .build();

        container.append(&back_btn);
        container.append(&title_label);

        Self {
            container,
            back_btn,
        }
    }

    pub fn connect_back<F: Fn() + 'static>(&self, f: F) {
        self.back_btn.connect_clicked(move |_| f());
    }
}

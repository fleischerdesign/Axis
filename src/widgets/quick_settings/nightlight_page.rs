use crate::app_context::AppContext;
use crate::services::nightlight::NightlightCmd;
use crate::widgets::quick_settings::components::{QsListRow, QsTile};
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct NightlightPage {
    pub container: gtk4::Box,
}

impl NightlightPage {
    pub fn new(ctx: AppContext, back_callback: impl Fn() + 'static, nl_tile: Rc<QsTile>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 16);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        let back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["qs-back-btn".to_string()])
            .build();
        let title = gtk4::Label::builder()
            .label("Nightlight")
            .halign(gtk4::Align::Start)
            .css_classes(vec!["qs-subpage-title".to_string()])
            .build();
        header.append(&back_btn);
        header.append(&title);

        container.append(&header);
        //container.append(&scrolled);
        container.set_vexpand(true);

        back_btn.connect_clicked(move |_| back_callback());

        Self { container }
    }
}

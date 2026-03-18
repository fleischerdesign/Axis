pub mod toast;
pub mod archive;

use gtk4::prelude::*;
use crate::services::notifications::Notification;

pub struct NotificationCard {
    pub container: gtk4::Box,
}

impl NotificationCard {
    pub fn new(data: &Notification) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        container.add_css_class("notification-card");
        container.set_width_request(380);
        container.set_hexpand(false);
        container.set_halign(gtk4::Align::End);

        // --- HEADER (App Name & Zeit) ---
        let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        header_box.add_css_class("notification-header");

        let app_icon = if !data.app_icon.is_empty() {
            gtk4::Image::from_icon_name(&data.app_icon)
        } else {
            gtk4::Image::from_icon_name("dialog-information-symbolic")
        };
        app_icon.set_pixel_size(16);
        
        let app_label = gtk4::Label::builder()
            .label(&data.app_name)
            .halign(gtk4::Align::Start)
            .css_classes(vec!["notification-app-name".to_string()])
            .build();

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        let time_label = gtk4::Label::builder()
            .label("jetzt")
            .halign(gtk4::Align::End)
            .css_classes(vec!["notification-time".to_string()])
            .build();

        header_box.append(&app_icon);
        header_box.append(&app_label);
        header_box.append(&spacer);
        header_box.append(&time_label);

        // --- CONTENT (Titel & Body) ---
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        content_box.add_css_class("notification-content-area");

        let summary_label = gtk4::Label::builder()
            .label(&data.summary)
            .halign(gtk4::Align::Start)
            .css_classes(vec!["notification-summary".to_string()])
            .wrap(true)
            .xalign(0.0)
            .build();

        let body_label = gtk4::Label::builder()
            .label(&data.body)
            .halign(gtk4::Align::Start)
            .css_classes(vec!["notification-body".to_string()])
            .wrap(true)
            .xalign(0.0)
            .build();

        content_box.append(&summary_label);
        if !data.body.is_empty() {
            content_box.append(&body_label);
        }

        container.append(&header_box);
        container.append(&content_box);

        Self { container }
    }
}

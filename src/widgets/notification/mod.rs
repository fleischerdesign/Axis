pub mod toast;
pub mod archive;

use gtk4::prelude::*;
use crate::services::notifications::Notification;
use crate::app_context::AppContext;
use crate::services::notifications::server::NotificationCmd;

pub struct NotificationCard {
    pub container: gtk4::Box,
}

impl NotificationCard {
    pub fn new(data: &Notification, ctx: AppContext) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        container.add_css_class("notification-card");
        container.set_width_request(380);
        container.set_hexpand(false);
        container.set_halign(gtk4::Align::End);

        // --- HEADER (App Name, Zeit, Close) ---
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

        // Close Button
        let close_btn = gtk4::Button::builder()
            .icon_name("window-close-symbolic")
            .css_classes(vec!["notification-close-btn".to_string()])
            .halign(gtk4::Align::End)
            .build();

        let id = data.id;
        let tx = ctx.notifications_tx.clone();
        close_btn.connect_clicked(move |_| {
            let _ = tx.send_blocking(NotificationCmd::Close(id));
        });

        header_box.append(&app_icon);
        header_box.append(&app_label);
        header_box.append(&spacer);
        header_box.append(&close_btn);

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

        // --- CLICK GESTURE (Default Action) ---
        let click = gtk4::GestureClick::new();
        let tx = ctx.notifications_tx.clone();
        let id = data.id;
        let has_default = data.actions.iter().any(|a| a.key == "default");
        
        click.connect_pressed(move |_, _, _, _| {
            if has_default {
                let _ = tx.send_blocking(NotificationCmd::Action(id, "default".to_string()));
                let _ = tx.send_blocking(NotificationCmd::Close(id));
            }
        });
        container.add_controller(click);

        // --- ACTIONS ---
        if !data.actions.is_empty() {
            let action_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            action_box.set_margin_top(8);
            action_box.set_homogeneous(true);

            for action in &data.actions {
                let btn = gtk4::Button::builder()
                    .label(&action.label)
                    .css_classes(vec!["notification-action-btn".to_string()])
                    .build();
                
                let tx = ctx.notifications_tx.clone();
                let key = action.key.clone();
                btn.connect_clicked(move |_| {
                    let _ = tx.send_blocking(NotificationCmd::Action(id, key.clone()));
                    let _ = tx.send_blocking(NotificationCmd::Close(id));
                });
                action_box.append(&btn);
            }
            container.append(&action_box);
        }

        Self { container }
    }
}

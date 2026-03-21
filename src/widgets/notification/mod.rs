pub mod archive;
pub mod toast;

use crate::app_context::AppContext;
use crate::services::notifications::server::NotificationCmd;
use crate::services::notifications::Notification;
use crate::widgets::components::swipe_dismiss::SwipeDismiss;
use gtk4::prelude::*;

fn format_time(timestamp: i64) -> String {
    let now = chrono::Local::now().timestamp();
    let diff = now - timestamp;

    if diff < 60 {
        "jetzt".to_string()
    } else if diff < 3600 {
        format!("vor {} Min", diff / 60)
    } else if diff < 86400 {
        format!("vor {} Std", diff / 3600)
    } else {
        format!("vor {} Tg", diff / 86400)
    }
}

pub struct NotificationCard {
    pub container: gtk4::Box,
}

impl NotificationCard {
    pub fn new(data: &Notification, ctx: AppContext) -> Self {
        // --- INNERE KARTE ---
        let card = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        card.add_css_class("notification-card");

        // --- HEADER ---
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
            .label(&format_time(data.timestamp))
            .halign(gtk4::Align::End)
            .css_classes(vec!["notification-time".to_string()])
            .build();

        let time_label_weak = time_label.downgrade();
        let timestamp = data.timestamp;
        gtk4::glib::timeout_add_seconds_local(60, move || {
            if let Some(label) = time_label_weak.upgrade() {
                label.set_label(&format_time(timestamp));
                gtk4::glib::ControlFlow::Continue
            } else {
                gtk4::glib::ControlFlow::Break
            }
        });

        let close_btn = gtk4::Button::builder()
            .icon_name("window-close-symbolic")
            .css_classes(vec!["notification-close-btn".to_string()])
            .halign(gtk4::Align::End)
            .build();

        let id = data.id;
        let tx = ctx.notifications.tx.clone();
        close_btn.connect_clicked(move |_| {
            let _ = tx.try_send(NotificationCmd::Close(id));
        });

        header_box.append(&app_icon);
        header_box.append(&app_label);
        header_box.append(&spacer);
        header_box.append(&time_label);
        header_box.append(&close_btn);

        // --- CONTENT ---
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

        card.append(&header_box);
        card.append(&content_box);

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

                let tx = ctx.notifications.tx.clone();
                let key = action.key.clone();
                let id_action = data.id;
                btn.connect_clicked(move |_| {
                    let _ = tx.try_send(NotificationCmd::Action(id_action, key.clone()));
                    let _ = tx.try_send(NotificationCmd::Close(id_action));
                });
                action_box.append(&btn);
            }
            card.append(&action_box);
        }

        // --- SWIPE TO DISMISS (generisch) ---
        let tx_swipe = ctx.notifications.tx.clone();
        let id_swipe = data.id;
        let swipe = SwipeDismiss::new(&card, move || {
            let _ = tx_swipe.try_send(NotificationCmd::Close(id_swipe));
        });

        // Wrapper-Eigenschaften auf den SwipeDismiss-Container
        let container = swipe.container.clone();
        container.set_width_request(380);
        container.set_hexpand(false);
        container.set_halign(gtk4::Align::End);
        container.add_css_class("notification-wrapper");

        // --- CLICK GESTURE (Default Action) ---
        let click = gtk4::GestureClick::new();
        let tx_click = ctx.notifications.tx.clone();
        let id_click = data.id;
        let has_default = data.actions.iter().any(|a| a.key == "default");

        click.connect_pressed(move |_, _, _, _| {
            if swipe.is_dragging() {
                return;
            }
            if has_default {
                let _ = tx_click.try_send(NotificationCmd::Action(id_click, "default".to_string()));
                let _ = tx_click.try_send(NotificationCmd::Close(id_click));
            }
        });
        container.add_controller(click);

        Self { container }
    }
}

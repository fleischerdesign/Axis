use gtk4::prelude::*;
use axis_domain::models::notifications::Notification;
use std::rc::Rc;
use crate::widgets::components::swipe_dismiss::SwipeDismiss;

pub type CloseCallback = Rc<dyn Fn(u32)>;
pub type ActionCallback = Rc<dyn Fn(u32, String, Option<String>)>;

pub fn format_time(timestamp: i64) -> String {
    let now = chrono::Local::now().timestamp();
    let diff = now - timestamp;

    if diff < 60 {
        "now".to_string()
    } else if diff < 3600 {
        format!("{} min ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hr ago", diff / 3600)
    } else {
        format!("{} d ago", diff / 86400)
    }
}

pub struct NotificationCard {
    pub container: gtk4::Box,
    pub time_label: gtk4::Label,
    pub timestamp: i64,
}

impl NotificationCard {
    pub fn new(
        data: &Notification,
        on_close: CloseCallback,
        on_action: ActionCallback,
    ) -> Self {
        let card = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        card.add_css_class("notification-card");
        card.set_hexpand(true);

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

        let close_btn = gtk4::Button::builder()
            .icon_name("window-close-symbolic")
            .css_classes(vec!["notification-close-btn".to_string()])
            .halign(gtk4::Align::End)
            .build();

        let close_id = data.id;
        let close_cb = on_close.clone();
        close_btn.connect_clicked(move |_| {
            close_cb(close_id);
        });

        header_box.append(&app_icon);
        header_box.append(&app_label);
        header_box.append(&spacer);
        header_box.append(&time_label);
        header_box.append(&close_btn);

        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        content_box.add_css_class("notification-content-area");

        let summary_label = gtk4::Label::builder()
            .label(&data.summary)
            .halign(gtk4::Align::Start)
            .css_classes(vec!["notification-summary".to_string()])
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .xalign(0.0)
            .build();

        let body_label = gtk4::Label::builder()
            .label(&data.body)
            .halign(gtk4::Align::Start)
            .css_classes(vec!["notification-body".to_string()])
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .xalign(0.0)
            .build();

        content_box.append(&summary_label);
        if !data.body.is_empty() {
            content_box.append(&body_label);
        }

        card.append(&header_box);
        card.append(&content_box);

        let entry: Option<gtk4::Entry> = data.input_placeholder.as_ref().map(|placeholder| {
            let e = gtk4::Entry::builder()
                .placeholder_text(placeholder)
                .css_classes(vec!["notification-input".to_string()])
                .build();
            e.set_margin_top(4);
            card.append(&e);
            e
        });

        if !data.actions.is_empty() {
            let action_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            action_box.set_margin_top(8);
            action_box.set_homogeneous(true);

            for action in &data.actions {
                let btn = gtk4::Button::builder()
                    .label(&action.label)
                    .css_classes(vec!["notification-action-btn".to_string()])
                    .build();

                let act_id = data.id;
                let act_key = action.key.clone();
                let act_cb = on_action.clone();
                if let Some(ref entry) = entry {
                    let entry_for_btn = entry.clone();
                    btn.connect_clicked(move |_| {
                        let text = entry_for_btn.buffer().text();
                        let input = if text.is_empty() { None } else { Some(text.to_string()) };
                        act_cb(act_id, act_key.clone(), input);
                    });
                } else {
                    btn.connect_clicked(move |_| {
                        act_cb(act_id, act_key.clone(), None);
                    });
                }
                action_box.append(&btn);
            }
            card.append(&action_box);
        }

        let has_default = data.actions.iter().any(|a| a.key == "default");
        if has_default {
            let click = gtk4::GestureClick::new();
            let click_id = data.id;
            let click_cb = on_action.clone();
            click.connect_pressed(move |_, _, _, _| {
                click_cb(click_id, "default".to_string(), None);
            });
            card.add_controller(click);
        }

        let close_id_swipe = data.id;
        let close_cb_swipe = on_close.clone();
        let swipe = SwipeDismiss::new(&card, move || {
            close_cb_swipe(close_id_swipe);
        });

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.set_halign(gtk4::Align::Fill);
        container.set_hexpand(true);
        container.add_css_class("notification-wrapper");
        container.append(&swipe.container);

        Self {
            container,
            time_label,
            timestamp: data.timestamp,
        }
    }
}

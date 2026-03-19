pub mod archive;
pub mod toast;

use crate::app_context::AppContext;
use crate::services::notifications::server::NotificationCmd;
use crate::services::notifications::Notification;
use gtk4::prelude::*;

// Helper für relative Zeitstempel
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
        // WRAPPER: Ein statischer Container, der Wischgesten auffängt, selbst wenn
        // die innere Karte sich durch CSS-Transforms darunter wegbewegt.
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.set_width_request(380);
        container.set_hexpand(false);
        container.set_halign(gtk4::Align::End);

        // Fast unsichtbarer Hintergrund, damit GTK den Bereich als "klickbar" ansieht
        let wrap_provider = gtk4::CssProvider::new();
        wrap_provider
            .load_from_string(".notification-wrapper { background-color: rgba(0, 0, 0, 0.01); }");
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("No display"),
            &wrap_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
        container.add_css_class("notification-wrapper");

        // INNERE KARTE: Das eigentliche visuelle Widget
        let card = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        card.add_css_class("notification-card");

        let instance_class = format!("notification-card-{}", data.id);
        card.add_css_class(&instance_class);

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

        // Live Zeitstempel
        let time_label = gtk4::Label::builder()
            .label(&format_time(data.timestamp))
            .halign(gtk4::Align::End)
            .css_classes(vec!["notification-time".to_string()])
            .build();

        // Memory-safe Timer (Aktualisiert jede Minute, stoppt wenn Widget gelöscht wird)
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

        // Close Button
        let close_btn = gtk4::Button::builder()
            .icon_name("window-close-symbolic")
            .css_classes(vec!["notification-close-btn".to_string()])
            .halign(gtk4::Align::End)
            .build();

        let id = data.id;
        let tx = ctx.notifications_tx.clone();
        close_btn.connect_clicked(move |_| {
            let _ = tx.try_send(NotificationCmd::Close(id));
        });

        header_box.append(&app_icon);
        header_box.append(&app_label);
        header_box.append(&spacer);
        header_box.append(&time_label);
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

                let tx = ctx.notifications_tx.clone();
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

        // Karte in den Wrapper legen
        container.append(&card);

        // State flags for gestures
        let was_swiped = std::rc::Rc::new(std::cell::Cell::new(false));
        let is_dragging = std::rc::Rc::new(std::cell::Cell::new(false));

        // --- CLICK GESTURE (Default Action) ---
        let click = gtk4::GestureClick::new();
        let tx_click = ctx.notifications_tx.clone();
        let id_click = data.id;
        let has_default = data.actions.iter().any(|a| a.key == "default");

        let was_swiped_click = was_swiped.clone();
        click.connect_pressed(move |_, _, _, _| {
            if was_swiped_click.get() {
                return;
            }
            if has_default {
                let _ = tx_click.try_send(NotificationCmd::Action(id_click, "default".to_string()));
                let _ = tx_click.try_send(NotificationCmd::Close(id_click));
            }
        });
        container.add_controller(click);

        // --- SWIPE TO DISMISS GESTURES ---
        // Der Provider wird auf die innere Karte angewandt
        let provider = gtk4::CssProvider::new();
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("No display"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );

        // 1. MOUSE / TOUCH DRAG (Klicken & Ziehen)
        let drag = gtk4::GestureDrag::new();
        drag.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let tx_drag = ctx.notifications_tx.clone();
        let id_drag = data.id;

        let is_dragging_drag = is_dragging.clone();
        drag.connect_drag_begin(move |_, _, _| {
            is_dragging_drag.set(true);
        });

        let provider_c = provider.clone();
        let class_c = instance_class.clone();
        drag.connect_drag_update(move |_, offset_x, _| {
            let x = offset_x.clamp(-380.0, 380.0);
            let progress = (x.abs() / 150.0).clamp(0.0, 1.0);
            let opacity = 1.0 - progress;

            let css = format!(
                ".{} {{ transform: translateX({}px); opacity: {:.2}; transition: none; }}",
                class_c,
                x.round() as i32,
                opacity
            );
            provider_c.load_from_string(css.as_str());
        });

        let provider_drag_end = provider.clone();
        let class_drag_end = instance_class.clone();
        let was_swiped_drag = was_swiped.clone();
        let is_dragging_end = is_dragging.clone();

        drag.connect_drag_end(move |_, offset_x, _| {
            is_dragging_end.set(false);
            
            if offset_x.abs() > 100.0 {
                was_swiped_drag.set(true);
                let ws_reset = was_swiped_drag.clone();
                gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                    ws_reset.set(false);
                });
                
                let _ = tx_drag.try_send(NotificationCmd::Close(id_drag));
            } else {
                let css = format!(
                    ".{} {{ transform: translateX(0px); opacity: 1.0; transition: all 0.2s ease-out; }}", 
                    class_drag_end
                );
                provider_drag_end.load_from_string(css.as_str());
            }
        });
        container.add_controller(drag);

        // 2. TRACKPAD SWIPE (2-Finger Scroll für Touchpads)
        let scroll = gtk4::EventControllerScroll::new(
            gtk4::EventControllerScrollFlags::HORIZONTAL
                | gtk4::EventControllerScrollFlags::KINETIC,
        );
        scroll.set_propagation_phase(gtk4::PropagationPhase::Capture);
        scroll.connect_scroll_begin(|_| {});

        let acc_dx = std::rc::Rc::new(std::cell::Cell::new(0.0));
        let scroll_timeout =
            std::rc::Rc::new(std::cell::RefCell::new(None::<gtk4::glib::SourceId>));
        let tx_scroll = ctx.notifications_tx.clone();
        let id_scroll = data.id;

        let acc_dx_c = acc_dx.clone();
        let provider_scroll_c = provider.clone();
        let timeout_c = scroll_timeout.clone();
        let class_scroll = instance_class.clone();
        let is_dragging_scroll = is_dragging.clone();

        scroll.connect_scroll(move |_, dx, _| {
            if is_dragging_scroll.get() {
                return gtk4::glib::Propagation::Stop;
            }

            let mut current = acc_dx_c.get();
            current -= dx * 3.0; 
            
            let x = current.clamp(-380.0, 380.0);
            acc_dx_c.set(x);

            let progress = (x.abs() / 150.0).clamp(0.0, 1.0);
            let opacity = 1.0 - progress;
            
            let css = format!(
                ".{} {{ transform: translateX({}px); opacity: {:.2}; transition: none; }}", 
                class_scroll, x.round() as i32, opacity
            );
            provider_scroll_c.load_from_string(css.as_str());

            if x.abs() > 120.0 {
                let _ = tx_scroll.try_send(NotificationCmd::Close(id_scroll));
                return gtk4::glib::Propagation::Stop;
            }

            if let Some(src) = timeout_c.borrow_mut().take() {
                src.remove();
            }

            let acc_end = acc_dx_c.clone();
            let prov_end = provider_scroll_c.clone();
            let timeout_end = timeout_c.clone();
            let class_scroll_end = class_scroll.clone();
            
            let src = gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
                let css = format!(
                    ".{} {{ transform: translateX(0px); opacity: 1.0; transition: all 0.3s ease-out; }}", 
                    class_scroll_end
                );
                prov_end.load_from_string(css.as_str());
                acc_end.set(0.0);
                *timeout_end.borrow_mut() = None;
            });
            *timeout_c.borrow_mut() = Some(src);

            gtk4::glib::Propagation::Stop
        });

        let provider_end = provider.clone();
        let class_end = instance_class.clone();
        scroll.connect_scroll_end(move |_| {
            let css = format!(
                ".{} {{ transform: translateX(0px); opacity: 1.0; transition: all 0.3s ease-out; }}", 
                class_end
            );
            provider_end.load_from_string(css.as_str());
            acc_dx.set(0.0);
        });

        container.add_controller(scroll);

        let scroll_timeout_destroy = scroll_timeout.clone();
        container.connect_destroy(move |_| {
            if let Some(src) = scroll_timeout_destroy.borrow_mut().take() {
                src.remove();
            }
        });

        Self { container }
    }
}

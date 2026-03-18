pub mod toast;
pub mod archive;

use gtk4::prelude::*;
use crate::services::notifications::Notification;
use crate::app_context::AppContext;
use crate::services::notifications::server::NotificationCmd;

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
            let _ = tx.send_blocking(NotificationCmd::Close(id));
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

        // --- SWIPE GESTURE (Wisch-zum-Löschen mit visueller Bewegung) ---
        let drag = gtk4::GestureDrag::new();
        let tx_swipe = ctx.notifications_tx.clone();
        let id_swipe = data.id;

        // Wir nutzen einen eigenen CSS-Provider für dieses spezifische Widget,
        // um GPU-beschleunigte CSS-Transforms während des Ziehens anzuwenden.
        let provider = gtk4::CssProvider::new();
        container.style_context().add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_USER);

        let provider_c = provider.clone();
        drag.connect_drag_update(move |_, offset_x, _| {
            let x = offset_x.clamp(-380.0, 380.0);
            // Je weiter man zieht, desto transparenter wird die Karte
            let progress = (x.abs() / 150.0).clamp(0.0, 1.0);
            let opacity = 1.0 - progress;
            
            let css = format!(
                ".notification-card {{ transform: translateX({}px); opacity: {:.2}; transition: none; }}", 
                x.round() as i32, opacity
            );
            provider_c.load_from_data(css.as_str());
        });

        let provider_end = provider.clone();
        drag.connect_drag_end(move |_, offset_x, _| {
            if offset_x.abs() > 100.0 {
                // Weit genug gezogen -> Löschen
                let _ = tx_swipe.send_blocking(NotificationCmd::Close(id_swipe));
            } else {
                // Abgebrochen -> Sanft zurückschnappen lassen
                provider_end.load_from_data(
                    ".notification-card { transform: translateX(0px); opacity: 1.0; transition: all 0.2s ease-out; }"
                );
            }
        });
        container.add_controller(drag);

        // --- TRACKPAD SWIPE (2-Finger Scroll für Touchpads) ---
        // Wir nutzen einen Debounce-Timer, da `scroll_end` bei Touchpads unzuverlässig ist.
        let scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::HORIZONTAL);
        let acc_dx = std::rc::Rc::new(std::cell::Cell::new(0.0));
        let scroll_timeout = std::rc::Rc::new(std::cell::RefCell::new(None::<gtk4::glib::SourceId>));
        let tx_scroll = ctx.notifications_tx.clone();
        let id_scroll = data.id;
        let provider_scroll = provider.clone();
        
        let acc_dx_c = acc_dx.clone();
        let provider_scroll_c = provider_scroll.clone();
        let timeout_c = scroll_timeout.clone();
        
        scroll.connect_scroll(move |_, dx, _| {
            let mut current = acc_dx_c.get();
            current -= dx * 4.0; // Multiplikator für sanftes Gefühl
            
            let x = current.clamp(-380.0, 380.0);
            acc_dx_c.set(x);

            let progress = (x.abs() / 150.0).clamp(0.0, 1.0);
            let opacity = 1.0 - progress;
            
            let css = format!(
                ".notification-card {{ transform: translateX({}px); opacity: {:.2}; transition: none; }}", 
                x.round() as i32, opacity
            );
            provider_scroll_c.load_from_data(css.as_str());

            // Vorherigen Timer abbrechen
            if let Some(src) = timeout_c.borrow_mut().take() {
                src.remove();
            }

            // Neuen Debounce-Timer setzen
            let acc_end = acc_dx_c.clone();
            let prov_end = provider_scroll_c.clone();
            let tx_end = tx_scroll.clone();
            let id_end = id_scroll;
            let timeout_end = timeout_c.clone();
            
            let src = gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                let final_x = acc_end.get();
                if final_x.abs() > 80.0 {
                    let _ = tx_end.send_blocking(NotificationCmd::Close(id_end));
                } else {
                    prov_end.load_from_data(
                        ".notification-card { transform: translateX(0px); opacity: 1.0; transition: all 0.2s ease-out; }"
                    );
                }
                acc_end.set(0.0);
                *timeout_end.borrow_mut() = None; // Sicherstellen, dass kein alter Timer gelöscht wird
            });
            *timeout_c.borrow_mut() = Some(src);

            gtk4::glib::Propagation::Stop
        });

        // Eventuelle Reste von scroll_end sicherheitshalber aufräumen
        let acc_dx_end = acc_dx.clone();
        scroll.connect_scroll_end(move |_| {
            acc_dx_end.set(0.0);
        });
        container.add_controller(scroll);

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

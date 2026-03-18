use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::app_context::AppContext;
use crate::widgets::notification::NotificationCard;
use std::time::Duration;
use std::rc::Rc;
use std::cell::Cell;

pub struct NotificationToastManager;

impl NotificationToastManager {
    pub fn init(app: &libadwaita::Application, ctx: AppContext) {
        let app_c = app.clone();
        
        let last_shown_id = Rc::new(Cell::new(0u32));
        
        ctx.notifications.subscribe(move |data| {
            if data.last_id > last_shown_id.get() {
                if let Some(n) = data.notifications.iter().find(|n| n.id == data.last_id) {
                    last_shown_id.set(data.last_id);
                    Self::show_toast(&app_c, n);
                }
            }
        });
    }

    fn show_toast(app: &libadwaita::Application, data: &crate::services::notifications::Notification) {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title(format!("Toast {}", data.id))
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 20);
        window.set_margin(Edge::Right, 10);
        window.set_exclusive_zone(-1);

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();

        let card = NotificationCard::new(data);
        revealer.set_child(Some(&card.container));
        window.set_child(Some(&revealer));
        window.present();
        revealer.set_reveal_child(true);

        // Auto-close sequence
        let window_c = window.clone();
        let revealer_c = revealer.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_secs(5), move || {
            revealer_c.set_reveal_child(false);
            gtk4::glib::timeout_add_local_once(Duration::from_millis(280), move || {
                window_c.close();
            });
        });

        // Close on click
        let click = gtk4::GestureClick::new();
        let window_c = window.clone();
        let revealer_c = revealer.clone();
        click.connect_pressed(move |_, _, _, _| {
            revealer_c.set_reveal_child(false);
            let win = window_c.clone();
            gtk4::glib::timeout_add_local_once(Duration::from_millis(280), move || {
                win.close();
            });
        });
        card.container.add_controller(click);
    }
}

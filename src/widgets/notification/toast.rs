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
        
        // Wir merken uns die ID der letzten Nachricht, um nur bei ECHTEN neuen Events zu toasten
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
        window.set_margin(Edge::Right, 20);
        window.set_exclusive_zone(-1);

        let card = NotificationCard::new(data);
        window.set_child(Some(&card.container));
        window.present();

        // Nach 5 Sekunden automatisch schließen
        let window_c = window.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_secs(5), move || {
            window_c.close();
        });

        // Schließen bei Klick
        let click = gtk4::GestureClick::new();
        let window_c = window.clone();
        click.connect_pressed(move |_, _, _, _| {
            window_c.close();
        });
        card.container.add_controller(click);
    }
}

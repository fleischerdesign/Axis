use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::app_context::AppContext;
use crate::widgets::notification::NotificationCard;
use std::time::Duration;
use std::rc::Rc;
use std::cell::{Cell, RefCell};

pub struct NotificationToastManager {
    window: gtk4::ApplicationWindow,
    container: gtk4::Box,
    last_shown_id: Rc<Cell<u32>>,
}

impl NotificationToastManager {
    pub fn init(app: &libadwaita::Application, ctx: AppContext) {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Notification Toasts")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 10);
        window.set_margin(Edge::Right, 10);
        window.set_exclusive_zone(-1);

        // Container für die Stapelung (Vertical Box)
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        container.set_valign(gtk4::Align::Start);
        window.set_child(Some(&container));

        let manager = Rc::new(Self {
            window,
            container,
            last_shown_id: Rc::new(Cell::new(0)),
        });

        let manager_c = manager.clone();
        ctx.notifications.subscribe(move |data| {
            if data.last_id > manager_c.last_shown_id.get() {
                if let Some(n) = data.notifications.iter().find(|n| n.id == data.last_id) {
                    manager_c.last_shown_id.set(data.last_id);
                    manager_c.add_toast(n);
                }
            }
        });
    }

    fn add_toast(&self, data: &crate::services::notifications::Notification) {
        let card = NotificationCard::new(data);
        
        // Revealer für die Ein- und Ausblend-Animation
        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();

        revealer.set_child(Some(&card.container));
        self.container.append(&revealer);
        
        // Fenster anzeigen, falls es noch versteckt war
        if !self.window.is_visible() {
            self.window.set_visible(true);
        }

        // Animation starten
        revealer.set_reveal_child(true);

        // Auto-Entfernung nach 5 Sekunden
        let container_c = self.container.clone();
        let revealer_c = revealer.clone();
        let window_c = self.window.clone();
        
        gtk4::glib::timeout_add_local_once(Duration::from_secs(5), move || {
            Self::remove_toast(&container_c, &revealer_c, &window_c);
        });

        // Entfernung bei Klick
        let click = gtk4::GestureClick::new();
        let container_click = self.container.clone();
        let revealer_click = revealer.clone();
        let window_click = self.window.clone();
        click.connect_pressed(move |_, _, _, _| {
            Self::remove_toast(&container_click, &revealer_click, &window_click);
        });
        card.container.add_controller(click);
    }

    fn remove_toast(container: &gtk4::Box, revealer: &gtk4::Revealer, window: &gtk4::ApplicationWindow) {
        revealer.set_reveal_child(false);
        
        let container_c = container.clone();
        let revealer_c = revealer.clone();
        let window_c = window.clone();
        
        gtk4::glib::timeout_add_local_once(Duration::from_millis(280), move || {
            container_c.remove(&revealer_c);
            
            // Wenn keine Toasts mehr da sind, Fenster verstecken
            if container_c.first_child().is_none() {
                window_c.set_visible(false);
            }
        });
    }
}

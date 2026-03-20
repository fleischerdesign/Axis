use crate::app_context::AppContext;
use crate::constants::REVEALER_TRANSITION_MS;
use crate::widgets::notification::NotificationCard;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

pub struct NotificationToastManager {
    window: gtk4::ApplicationWindow,
    container: gtk4::Box,
    last_shown_id: Cell<u32>,
    active_toasts: RefCell<HashMap<u32, gtk4::Revealer>>,
    ctx: AppContext,
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

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        container.set_valign(gtk4::Align::Start);
        window.set_child(Some(&container));

        let manager = Rc::new(Self {
            window,
            container,
            last_shown_id: Cell::new(0),
            active_toasts: RefCell::new(HashMap::new()),
            ctx: ctx.clone(),
        });

        let manager_c = manager.clone();
        ctx.notifications.subscribe(move |data| {
            manager_c.sync(data);
        });
    }

    fn sync(&self, data: &crate::services::notifications::NotificationData) {
        // 1. Neue Toasts hinzufügen
        if data.last_id > self.last_shown_id.get() {
            if let Some(n) = data.notifications.iter().find(|n| n.id == data.last_id) {
                self.last_shown_id.set(data.last_id);
                self.add_toast(n);
            }
        }

        // 2. Nicht mehr vorhandene Toasts entfernen (Reaktive UI)
        let mut to_remove = Vec::new();
        for id in self.active_toasts.borrow().keys() {
            if !data.notifications.iter().any(|n| n.id == *id) {
                to_remove.push(*id);
            }
        }

        for id in to_remove {
            self.remove_toast_by_id(id);
        }
    }

    fn add_toast(&self, data: &crate::services::notifications::Notification) {
        let card = NotificationCard::new(data, self.ctx.clone());

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();

        revealer.set_child(Some(&card.container));
        self.container.append(&revealer);

        self.active_toasts
            .borrow_mut()
            .insert(data.id, revealer.clone());

        if !self.window.is_visible() {
            self.window.set_visible(true);
        }

        revealer.set_reveal_child(true);

        // Auto-Entfernung nach 5 Sekunden (NUR lokal als Toast verstecken, NICHT global löschen!)
        let id = data.id;
        let active_toasts_c = self.active_toasts.clone();
        let container_c = self.container.clone();
        let window_c = self.window.clone();

        gtk4::glib::timeout_add_local_once(Duration::from_secs(5), move || {
            if let Some(revealer) = active_toasts_c.borrow_mut().remove(&id) {
                revealer.set_reveal_child(false);

                let container_cc = container_c.clone();
                let window_cc = window_c.clone();

                gtk4::glib::timeout_add_local_once(
                    Duration::from_millis(REVEALER_TRANSITION_MS as u64),
                    move || {
                        container_cc.remove(&revealer);
                        if container_cc.first_child().is_none() {
                            window_cc.set_visible(false);
                        }
                    },
                );
            }
        });
    }

    fn remove_toast_by_id(&self, id: u32) {
        if let Some(revealer) = self.active_toasts.borrow_mut().remove(&id) {
            revealer.set_reveal_child(false);

            let container_c = self.container.clone();
            let window_c = self.window.clone();

            gtk4::glib::timeout_add_local_once(
                Duration::from_millis(REVEALER_TRANSITION_MS as u64),
                move || {
                    container_c.remove(&revealer);
                    if container_c.first_child().is_none() {
                        window_c.set_visible(false);
                    }
                },
            );
        }
    }
}

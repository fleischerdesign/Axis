use crate::app_context::AppContext;
use crate::widgets::components::revealer_handle;
use crate::widgets::notification::NotificationCard;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::Duration;

pub struct NotificationToastManager {
    window: gtk4::ApplicationWindow,
    container: gtk4::Box,
    last_shown_id: Cell<u32>,
    active_toasts: RefCell<HashMap<u32, gtk4::Revealer>>,
    ctx: AppContext,
}

thread_local! {
    static MANAGER: RefCell<Option<NotificationToastManager>> = RefCell::new(None);
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
        window.set_default_size(380, -1);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        container.set_valign(gtk4::Align::Start);
        window.set_child(Some(&container));

        MANAGER.with_borrow_mut(|m| {
            *m = Some(Self {
                window,
                container,
                last_shown_id: Cell::new(0),
                active_toasts: RefCell::new(HashMap::new()),
                ctx: ctx.clone(),
            });
        });

        ctx.notifications.subscribe(|data| {
            MANAGER.with_borrow(|m| {
                if let Some(mgr) = m {
                    mgr.sync(data);
                }
            });
        });
    }

    fn sync(&self, data: &crate::services::notifications::NotificationData) {
        let is_new = data.last_id > self.last_shown_id.get();
        let needs_show = data.last_id > 0
            && data.notifications.iter().any(|n| n.id == data.last_id)
            && !self.active_toasts.borrow().contains_key(&data.last_id);

        if is_new || needs_show {
            if let Some(n) = data.notifications.iter().find(|n| n.id == data.last_id) {
                if n.internal_id > 0 || !self.ctx.dnd.get().enabled {
                    self.last_shown_id.set(data.last_id);
                    self.add_toast(n);
                }
            }
        }

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

        let revealer = revealer_handle::create_revealer();
        revealer.set_child(Some(&card.container));
        self.container.append(&revealer);

        self.active_toasts
            .borrow_mut()
            .insert(data.id, revealer.clone());

        if !self.window.is_visible() {
            self.window.set_visible(true);
        }

        revealer.set_reveal_child(true);

        if data.internal_id == 0 {
            let id = data.id;
            let active_toasts_c = self.active_toasts.clone();
            let container_c = self.container.clone();
            let window_c = self.window.clone();

            gtk4::glib::timeout_add_local_once(Duration::from_secs(5), move || {
                if let Some(revealer) = active_toasts_c.borrow_mut().remove(&id) {
                    revealer_handle::animate_out(
                        &revealer,
                        &container_c,
                        Some({
                            let cc = container_c.clone();
                            let wc = window_c.clone();
                            move || {
                                if cc.first_child().is_none() {
                                    wc.set_visible(false);
                                }
                            }
                        }),
                    );
                }
            });
        }
    }

    fn remove_toast_by_id(&self, id: u32) {
        if let Some(revealer) = self.active_toasts.borrow_mut().remove(&id) {
            revealer_handle::animate_out(
                &revealer,
                &self.container,
                Some({
                    let cc = self.container.clone();
                    let wc = self.window.clone();
                    move || {
                        if cc.first_child().is_none() {
                            wc.set_visible(false);
                        }
                    }
                }),
            );
        }
    }
}

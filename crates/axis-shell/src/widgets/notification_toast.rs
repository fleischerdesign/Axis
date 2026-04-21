use gtk4::prelude::*;
use gtk4_layer_shell::{LayerShell, Layer, Edge};
use axis_domain::models::notifications::{Notification, NotificationStatus};
use axis_domain::models::dnd::DndStatus;
use crate::widgets::notification_card::{NotificationCard, CloseCallback, ActionCallback};
use axis_presentation::View;
use crate::presentation::notifications::NotificationPopupAware;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

pub struct NotificationToastManager {
    window: gtk4::ApplicationWindow,
    container: gtk4::Box,
    last_shown_id: Cell<u32>,
    active_toasts: RefCell<HashMap<u32, gtk4::Revealer>>,
    on_close: CloseCallback,
    on_action: ActionCallback,
    popup_open: Cell<bool>,
    dnd_enabled: Cell<bool>,
}

impl NotificationToastManager {
    pub fn new(
        app: &libadwaita::Application,
        on_close: CloseCallback,
        on_action: ActionCallback,
    ) -> Self {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .build();

        window.init_layer_shell();
        window.add_css_class("notification-toast-window");
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

        Self {
            window,
            container,
            last_shown_id: Cell::new(0),
            active_toasts: RefCell::new(HashMap::new()),
            on_close,
            on_action,
            popup_open: Cell::new(false),
            dnd_enabled: Cell::new(false),
        }
    }

    fn add_toast(&self, data: &Notification) {
        let card = NotificationCard::new(
            data,
            self.on_close.clone(),
            self.on_action.clone(),
        );

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
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

        let id = data.id;
        let auto_dismiss = !data.ignore_dnd && data.internal_id == 0;
        let active = RefCell::clone(&self.active_toasts);
        let cont = self.container.clone();
        let win = self.window.clone();

        if auto_dismiss {
            gtk4::glib::timeout_add_local_once(std::time::Duration::from_secs(5), move || {
                if let Some(r) = active.borrow_mut().remove(&id) {
                    r.set_reveal_child(false);
                    let cont_c = cont.clone();
                    let win_c = win.clone();
                    gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(260), move || {
                        cont_c.remove(&r);
                        if cont_c.first_child().is_none() {
                            win_c.set_visible(false);
                        }
                    });
                }
            });
        }
    }

    fn remove_toast_by_id(&self, id: u32) {
        if let Some(revealer) = self.active_toasts.borrow_mut().remove(&id) {
            let cont = self.container.clone();
            let win = self.window.clone();
            revealer.set_reveal_child(false);
            gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(260), move || {
                cont.remove(&revealer);
                if cont.first_child().is_none() {
                    win.set_visible(false);
                }
            });
        }
    }

    fn hide_all_toasts(&self) {
        let toasts: Vec<_> = self.active_toasts.borrow_mut().drain().collect();
        for (_, revealer) in toasts {
            let cont = self.container.clone();
            let win = self.window.clone();
            revealer.set_reveal_child(false);
            gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(260), move || {
                cont.remove(&revealer);
                if cont.first_child().is_none() {
                    win.set_visible(false);
                }
            });
        }
    }
}

impl View<NotificationStatus> for NotificationToastManager {
    fn render(&self, status: &NotificationStatus) {
        let is_new = status.last_id > self.last_shown_id.get();
        let needs_show = status.last_id > 0
            && status.notifications.iter().any(|n| n.id == status.last_id)
            && !self.active_toasts.borrow().contains_key(&status.last_id);

        if is_new || needs_show {
            if let Some(n) = status.notifications.iter().find(|n| n.id == status.last_id) {
                self.last_shown_id.set(status.last_id);
                if !self.popup_open.get() && (n.ignore_dnd || !self.dnd_enabled.get()) {
                    self.add_toast(n);
                }
            }
        }

        let active_ids: HashSet<u32> = status.notifications.iter().map(|n| n.id).collect();
        let to_remove: Vec<u32> = self.active_toasts.borrow()
            .keys()
            .filter(|id| !active_ids.contains(id))
            .copied()
            .collect();

        for id in to_remove {
            self.remove_toast_by_id(id);
        }
    }
}

impl View<DndStatus> for NotificationToastManager {
    fn render(&self, status: &DndStatus) {
        self.dnd_enabled.set(status.enabled);
    }
}

impl NotificationPopupAware for NotificationToastManager {
    fn set_popup_open(&self, open: bool) {
        self.popup_open.set(open);
        if open {
            self.hide_all_toasts();
        }
    }
}

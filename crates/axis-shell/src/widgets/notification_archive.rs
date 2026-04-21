use gtk4::prelude::*;
use axis_domain::models::notifications::NotificationStatus;
use crate::widgets::notification_card::{NotificationCard, CloseCallback, ActionCallback};
use crate::presentation::presenter::View;
use crate::presentation::notifications::NotificationPopupAware;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

pub struct NotificationArchive {
    pub container: gtk4::Revealer,
    list_box: gtk4::Box,
    active_items: RefCell<HashMap<u32, gtk4::Revealer>>,
    popup_open: Cell<bool>,
    on_close: CloseCallback,
    on_action: ActionCallback,
}

impl NotificationArchive {
    pub fn new(
        on_close: CloseCallback,
        on_action: ActionCallback,
    ) -> Self {
        let container = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .transition_duration(250)
            .build();

        let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        list_box.set_valign(gtk4::Align::End);
        container.set_margin_bottom(8);
        container.set_child(Some(&list_box));

        Self {
            container,
            list_box,
            active_items: RefCell::new(HashMap::new()),
            popup_open: Cell::new(false),
            on_close,
            on_action,
        }
    }
}

impl NotificationPopupAware for NotificationArchive {
    fn set_popup_open(&self, open: bool) {
        self.popup_open.set(open);
        if open && !self.active_items.borrow().is_empty() {
            self.container.set_reveal_child(true);
        } else if !open {
            self.container.set_reveal_child(false);
        }
    }
}

impl View<NotificationStatus> for NotificationArchive {
    fn render(&self, status: &NotificationStatus) {
        let mut active_items = self.active_items.borrow_mut();

        let active_ids: HashSet<u32> = status.notifications.iter().map(|n| n.id).collect();
        let to_remove: Vec<u32> = active_items
            .keys()
            .filter(|id| !active_ids.contains(id))
            .copied()
            .collect();

        for id in to_remove {
            if let Some(revealer) = active_items.remove(&id) {
                revealer.set_reveal_child(false);
                let lb = self.list_box.clone();
                gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(260), move || {
                    lb.remove(&revealer);
                });
            }
        }

        if active_items.is_empty() && status.notifications.is_empty() {
            self.container.set_reveal_child(false);
        }

        for n in &status.notifications {
            if !active_items.contains_key(&n.id) {
                let card = NotificationCard::new(
                    n,
                    self.on_close.clone(),
                    self.on_action.clone(),
                );

                let revealer = gtk4::Revealer::builder()
                    .transition_type(gtk4::RevealerTransitionType::SlideDown)
                    .transition_duration(250)
                    .build();
                revealer.set_child(Some(&card.container));
                self.list_box.append(&revealer);

                active_items.insert(n.id, revealer.clone());

                gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(10), move || {
                    revealer.set_reveal_child(true);
                });
            }
        }

        if self.popup_open.get() && !active_items.is_empty() && !self.container.reveals_child() {
            self.container.set_reveal_child(true);
        }
    }
}

use crate::app_context::AppContext;
use crate::constants::REVEALER_TRANSITION_MS;
use crate::widgets::components::revealer_handle;
use crate::widgets::notification::NotificationCard;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

pub struct NotificationArchiveManager {
    pub container: gtk4::Revealer,
    list_box: gtk4::Box,
    hide_timeout: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
    active_items: Rc<RefCell<HashMap<u32, gtk4::Revealer>>>,
    ctx: AppContext,
}

impl NotificationArchiveManager {
    pub fn new(ctx: AppContext) -> Rc<Self> {
        let container = revealer_handle::create_revealer();

        let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        list_box.set_valign(gtk4::Align::End);
        list_box.set_halign(gtk4::Align::End);
        list_box.set_width_request(380);
        container.set_child(Some(&list_box));

        let manager = Rc::new(Self {
            container,
            list_box,
            hide_timeout: Rc::new(RefCell::new(None)),
            active_items: Rc::new(RefCell::new(HashMap::new())),
            ctx: ctx.clone(),
        });

        let manager_c = manager.clone();
        ctx.notifications.subscribe(move |data| {
            manager_c.sync(data);
        });

        manager
    }

    pub fn set_visible(&self, visible: bool) {
        if visible {
            self.show_archive();
        } else {
            self.hide_archive();
        }
    }

    fn show_archive(&self) {
        if let Some(src) = self.hide_timeout.borrow_mut().take() {
            src.remove();
        }

        if self.active_items.borrow().is_empty() {
            return;
        }

        self.container.set_reveal_child(true);
    }

    fn hide_archive(&self) {
        if let Some(src) = self.hide_timeout.borrow_mut().take() {
            src.remove();
        }

        self.container.set_reveal_child(false);
    }

    fn sync(&self, data: &crate::services::notifications::NotificationData) {
        let mut active_items = self.active_items.borrow_mut();

        // Remove stale items
        let to_remove: Vec<u32> = active_items
            .keys()
            .filter(|id| !data.notifications.iter().any(|n| n.id == **id))
            .copied()
            .collect();

        for id in to_remove {
            if let Some(revealer) = active_items.remove(&id) {
                revealer_handle::animate_out(&revealer, &self.list_box, None::<fn()>);
            }
        }

        if active_items.is_empty() {
            self.hide_archive();
        }

        // Add new items
        for n in &data.notifications {
            if !active_items.contains_key(&n.id) {
                let card = NotificationCard::new(n, self.ctx.clone());

                let revealer = revealer_handle::create_revealer();
                revealer.set_child(Some(&card.container));
                self.list_box.append(&revealer);

                active_items.insert(n.id, revealer.clone());

                gtk4::glib::timeout_add_local_once(Duration::from_millis(10), move || {
                    revealer.set_reveal_child(true);
                });
            }
        }

        if !active_items.is_empty() && !self.container.reveals_child() {
            self.container.set_reveal_child(true);
        }
    }
}

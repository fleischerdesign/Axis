use crate::app_context::AppContext;
use crate::constants::REVEALER_TRANSITION_MS;
use crate::widgets::components::revealer_handle;
use crate::widgets::notification::NotificationCard;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

const ARCHIVE_MARGIN_BOTTOM: i32 = 76;

pub struct NotificationArchiveManager {
    window: gtk4::ApplicationWindow,
    main_revealer: gtk4::Revealer,
    list_box: gtk4::Box,
    qs_content: gtk4::Box,
    hide_timeout: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
    active_items: Rc<RefCell<HashMap<u32, gtk4::Revealer>>>,
    ctx: AppContext,
}

impl NotificationArchiveManager {
    pub fn new(app: &libadwaita::Application, ctx: AppContext, qs_content: &gtk4::Box) -> Rc<Self> {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Notification Archive")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Right, 10);
        window.set_exclusive_zone(-1);

        let main_revealer = revealer_handle::create_revealer();

        let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        list_box.set_valign(gtk4::Align::End);
        list_box.set_halign(gtk4::Align::End);
        list_box.set_width_request(380);
        main_revealer.set_child(Some(&list_box));
        window.set_child(Some(&main_revealer));

        let manager = Rc::new(Self {
            window,
            main_revealer,
            list_box,
            qs_content: qs_content.clone(),
            hide_timeout: Rc::new(RefCell::new(None)),
            active_items: Rc::new(RefCell::new(HashMap::new())),
            ctx: ctx.clone(),
        });

        // Dynamische Höhenanpassung via tick callback
        let window_c = manager.window.clone();
        let qs_c = manager.qs_content.clone();
        let last_height: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
        manager.window.add_tick_callback(move |_, _| {
            if window_c.is_visible() {
                let height = qs_c.height();
                let mut last = last_height.borrow_mut();
                if height > 50 && height != *last {
                    *last = height;
                    window_c.set_margin(Edge::Bottom, height + ARCHIVE_MARGIN_BOTTOM);
                }
            }
            gtk4::glib::ControlFlow::Continue
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

        let height = self.qs_content.height();
        if height > 50 {
            self.window
                .set_margin(Edge::Bottom, height + ARCHIVE_MARGIN_BOTTOM);
        }

        if self.active_items.borrow().is_empty() {
            return;
        }

        self.window.set_visible(true);
        let rev = self.main_revealer.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(10), move || {
            rev.set_reveal_child(true);
        });
    }

    fn hide_archive(&self) {
        if let Some(src) = self.hide_timeout.borrow_mut().take() {
            src.remove();
        }

        self.main_revealer.set_reveal_child(false);
        let win = self.window.clone();
        let hide_timeout_c = self.hide_timeout.clone();
        let src = gtk4::glib::timeout_add_local_once(
            Duration::from_millis(REVEALER_TRANSITION_MS as u64),
            move || {
                win.set_visible(false);
                *hide_timeout_c.borrow_mut() = None;
            },
        );
        *self.hide_timeout.borrow_mut() = Some(src);
    }

    fn sync(&self, data: &crate::services::notifications::NotificationData) {
        let mut active_items = self.active_items.borrow_mut();

        // 1. Alte/Gelöschte entfernen
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

        // 2. Neue hinzufügen
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

        if !active_items.is_empty()
            && self.window.is_visible()
            && !self.main_revealer.reveals_child()
        {
            self.main_revealer.set_reveal_child(true);
        }
    }
}

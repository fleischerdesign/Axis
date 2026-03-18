use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::app_context::AppContext;
use crate::widgets::notification::NotificationCard;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

pub struct NotificationArchiveManager {
    window: gtk4::ApplicationWindow,
    revealer: gtk4::Revealer,
    list_box: gtk4::Box,
    qs_content: gtk4::Box,
    hide_timeout: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
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

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .reveal_child(false) // Startet immer versteckt
            .build();

        let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        list_box.set_valign(gtk4::Align::End);
        list_box.set_halign(gtk4::Align::End);
        list_box.set_width_request(380);
        revealer.set_child(Some(&list_box));
        window.set_child(Some(&revealer));

        let manager = Rc::new(Self { 
            window, 
            revealer,
            list_box, 
            qs_content: qs_content.clone(),
            hide_timeout: Rc::new(RefCell::new(None)),
        });
        
        let manager_c = manager.clone();
        ctx.notifications.subscribe(move |data| {
            manager_c.update(&data.notifications);
        });

        manager
    }

    pub fn set_visible(&self, visible: bool) {
        // Bestehenden Hide-Timer immer abbrechen
        if let Some(src) = self.hide_timeout.borrow_mut().take() {
            src.remove();
        }

        if visible {
            let height = self.qs_content.allocated_height();
            if height > 100 {
                self.window.set_margin(Edge::Bottom, height + 54 + 12);
            } else {
                self.window.set_margin(Edge::Bottom, 500);
            }
            
            self.window.set_visible(true);
            // Kleiner Delay für den Revealer, damit das Fenster erst gemappt werden kann
            let rev = self.revealer.clone();
            gtk4::glib::timeout_add_local_once(Duration::from_millis(10), move || {
                rev.set_reveal_child(true);
            });
        } else {
            self.revealer.set_reveal_child(false);
            let win = self.window.clone();
            let hide_timeout_c = self.hide_timeout.clone();
            
            let src = gtk4::glib::timeout_add_local_once(Duration::from_millis(260), move || {
                win.set_visible(false);
                *hide_timeout_c.borrow_mut() = None;
            });
            *self.hide_timeout.borrow_mut() = Some(src);
        }
    }

    fn update(&self, notifications: &[crate::services::notifications::Notification]) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        for n in notifications.iter().rev().take(10) {
            let card = NotificationCard::new(n);
            self.list_box.append(&card.container);
        }
    }
}

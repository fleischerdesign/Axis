use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::app_context::AppContext;
use crate::widgets::notification::NotificationCard;
use std::rc::Rc;

pub struct NotificationArchiveManager {
    window: gtk4::ApplicationWindow,
    list_box: gtk4::Box,
    qs_content: gtk4::Box,
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
        window.set_margin(Edge::Right, 20);
        window.set_exclusive_zone(-1);

        let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        list_box.set_valign(gtk4::Align::End);
        window.set_child(Some(&list_box));

        let manager = Rc::new(Self { 
            window, 
            list_box, 
            qs_content: qs_content.clone() 
        });
        
        let manager_c = manager.clone();
        ctx.notifications.subscribe(move |data| {
            manager_c.update(&data.notifications);
        });

        manager
    }

    pub fn set_visible(&self, visible: bool) {
        if visible {
            // Wir aktualisieren das Margin JEDES MAL, wenn wir eingeblendet werden
            let height = self.qs_content.allocated_height();
            if height > 100 {
                self.window.set_margin(Edge::Bottom, height + 54 + 12);
            } else {
                self.window.set_margin(Edge::Bottom, 500); // Sicherer Fallback
            }
            self.window.present();
        } else {
            self.window.hide();
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

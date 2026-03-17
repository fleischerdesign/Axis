use crate::app_context::AppContext;
use crate::services::launcher::LauncherCmd;
use crate::widgets::quick_settings::components::QsListRow;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

pub struct LauncherPopup {
    pub window: gtk4::Window,
    pub is_open: Rc<RefCell<bool>>,
}

impl LauncherPopup {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Self {
        let is_open = Rc::new(RefCell::new(false));

        let window = gtk4::Window::builder()
            .application(app)
            .title("Carp Launcher")
            .visible(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_margin(Edge::Bottom, 64);
        window.set_margin(Edge::Left, 10);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.add_css_class("qs-panel");
        container.set_width_request(380);
        container.set_height_request(450);

        // --- SEARCH ENTRY ---
        let entry_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        entry_box.set_margin_bottom(12);
        
        let entry = gtk4::Entry::builder()
            .placeholder_text("Suchen, Finden, Machen...")
            .hexpand(true)
            .css_classes(vec!["qs-entry".to_string()])
            .build();
        entry_box.append(&entry);
        container.append(&entry_box);

        // --- RESULTS LIST ---
        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list".to_string()])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();
        scrolled.add_css_class("qs-scrolled");
        scrolled.set_child(Some(&list));
        container.append(&scrolled);

        // --- REAKTIVE BINDINGS ---
        
        // Suche triggern bei Texteingabe
        let tx = ctx.launcher_tx.clone();
        entry.connect_changed(move |e| {
            let _ = tx.send_blocking(LauncherCmd::Search(e.text().to_string()));
        });

        // Ergebnisse anzeigen
        let list_c = list.clone();
        ctx.launcher.subscribe(move |data| {
            // Liste leeren
            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }

            // Ergebnisse hinzufügen
            for item in &data.results {
                let row = QsListRow::new(
                    &item.title,
                    &item.icon_name,
                    false,
                    item.description.as_deref(),
                );
                list_c.append(&row.container);
            }
        });

        let qs_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();
        qs_revealer.set_child(Some(&container));
        window.set_child(Some(&qs_revealer));

        Self { window, is_open }
    }

    pub fn toggle(&self) {
        let mut open = self.is_open.borrow_mut();
        *open = !*open;
        let revealer = self
            .window
            .child()
            .and_then(|c| c.downcast::<gtk4::Revealer>().ok())
            .unwrap();
        
        if *open {
            self.window.set_visible(true);
            revealer.set_reveal_child(true);
            // Fokus auf das Entry legen
            if let Some(container) = revealer.child() {
                if let Some(box_w) = container.downcast_ref::<gtk4::Box>() {
                    if let Some(entry_box) = box_w.first_child() {
                        if let Some(entry) = entry_box.first_child() {
                            entry.grab_focus();
                        }
                    }
                }
            }
        } else {
            revealer.set_reveal_child(false);
            let win = self.window.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(280), move || {
                win.set_visible(false);
                gtk4::glib::ControlFlow::Break
            });
        }
    }
}

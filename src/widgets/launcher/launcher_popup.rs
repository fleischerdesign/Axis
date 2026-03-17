use crate::app_context::AppContext;
use crate::services::launcher::LauncherCmd;
use crate::widgets::ListRow;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

pub struct LauncherPopup {
    pub window: gtk4::Window,
    pub is_open: Rc<RefCell<bool>>,
    pub ctx: AppContext,
}

impl LauncherPopup {
    pub fn new(
        app: &libadwaita::Application,
        ctx: AppContext,
        on_state_change: impl Fn() + 'static,
    ) -> Self {
        let is_open = Rc::new(RefCell::new(false));
        let on_state_change = Rc::new(on_state_change);

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

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.add_css_class("qs-panel");
        container.set_width_request(380);
        container.set_height_request(450);

        // --- LEFT PANE ---
        let left_pane = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        left_pane.set_width_request(380);
        left_pane.set_hexpand(true);

        let entry_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        entry_box.set_margin_bottom(12);
        
        let entry = gtk4::Entry::builder()
            .placeholder_text("Suchen, Finden, Machen...")
            .hexpand(true)
            .css_classes(vec!["qs-entry".to_string()])
            .build();
        entry_box.append(&entry);
        left_pane.append(&entry_box);

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
        left_pane.append(&scrolled);
        container.append(&left_pane);

        // --- RIGHT PANE ---
        let detail_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideRight)
            .transition_duration(250)
            .build();

        let detail_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        detail_box.set_width_request(280);
        detail_box.set_margin_start(16);
        detail_box.set_margin_end(16);
        detail_box.add_css_class("launcher-details");

        let detail_title = gtk4::Label::builder()
            .halign(gtk4::Align::Start)
            .css_classes(vec!["qs-subpage-title".to_string()])
            .wrap(true)
            .build();
        let detail_desc = gtk4::Label::builder()
            .halign(gtk4::Align::Start)
            .wrap(true)
            .css_classes(vec!["list-sublabel".to_string()])
            .build();
        
        detail_box.append(&detail_title);
        detail_box.append(&detail_desc);
        detail_revealer.set_child(Some(&detail_box));
        container.append(&detail_revealer);

        // --- REAKTIVE BINDINGS ---
        let tx = ctx.launcher_tx.clone();
        entry.connect_changed(move |e| {
            let _ = tx.send_blocking(LauncherCmd::Search(e.text().to_string()));
        });

        let qs_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();
        qs_revealer.set_child(Some(&container));
        window.set_child(Some(&qs_revealer));

        // Keyboard Handling (Pfeiltasten)
        let tx_key = ctx.launcher_tx.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                gtk4::gdk::Key::Down => {
                    let _ = tx_key.send_blocking(LauncherCmd::SelectNext);
                    gtk4::glib::Propagation::Stop
                }
                gtk4::gdk::Key::Up => {
                    let _ = tx_key.send_blocking(LauncherCmd::SelectPrev);
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });
        entry.add_controller(key_controller);

        // Enter-Taste (Activate Signal)
        let tx_activate = ctx.launcher_tx.clone();
        let win_activate = window.clone();
        let is_open_activate = is_open.clone();
        let on_state_change_activate = on_state_change.clone();
        entry.connect_activate(move |_| {
            let _ = tx_activate.send_blocking(LauncherCmd::Activate);
            Self::close_internal(&win_activate, &is_open_activate);
            on_state_change_activate();
        });

        // Results Rendering
        let list_c = list.clone();
        let d_title = detail_title.clone();
        let d_desc = detail_desc.clone();
        let d_rev = detail_revealer.clone();
        let win_c = window.clone();
        let tx_click = ctx.launcher_tx.clone();
        let is_open_click = is_open.clone();
        let window_click = window.clone();
        let on_state_change_click = on_state_change.clone();

        ctx.launcher.subscribe(move |data| {
            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }

            for (i, item) in data.results.iter().enumerate() {
                let is_selected = data.selected_index == Some(i);
                let row = ListRow::new(
                    &item.title,
                    &item.icon_name,
                    is_selected,
                    item.description.as_deref(),
                    false,
                );
                
                let tx_row = tx_click.clone();
                let is_open_row = is_open_click.clone();
                let window_row = window_click.clone();
                let on_state_change_row = on_state_change_click.clone();
                
                row.button.connect_clicked(move |_| {
                    let _ = tx_row.send_blocking(LauncherCmd::Activate);
                    Self::close_internal(&window_row, &is_open_row);
                    on_state_change_row();
                });

                list_c.append(&row.container);

                if is_selected {
                    d_title.set_text(&item.title);
                    d_desc.set_text(item.description.as_deref().unwrap_or(""));
                    d_rev.set_reveal_child(true);
                    win_c.set_width_request(380 + 300);
                }
            }

            if data.results.is_empty() {
                d_rev.set_reveal_child(false);
                win_c.set_width_request(380);
            }
        });

        Self { window, is_open, ctx }
    }

    fn close_internal(window: &gtk4::Window, is_open: &Rc<RefCell<bool>>) {
        let mut open = is_open.borrow_mut();
        *open = false;
        let revealer = window
            .child()
            .and_then(|c| c.downcast::<gtk4::Revealer>().ok())
            .unwrap();
        
        revealer.set_reveal_child(false);
        let win = window.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(280), move || {
            win.set_visible(false);
            gtk4::glib::ControlFlow::Break
        });
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
            
            // Initiale Suche mit leerem String triggern
            let _ = self.ctx.launcher_tx.send_blocking(LauncherCmd::Search("".to_string()));

            if let Some(container) = revealer.child() {
                if let Some(box_w) = container.downcast_ref::<gtk4::Box>() {
                    if let Some(left_pane) = box_w.first_child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
                        if let Some(entry_box) = left_pane.first_child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
                            if let Some(entry) = entry_box.first_child().and_then(|c| c.downcast::<gtk4::Entry>().ok()) {
                                entry.set_text("");
                                entry.grab_focus();
                            }
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

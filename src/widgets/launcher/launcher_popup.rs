use crate::app_context::AppContext;
use crate::services::launcher::LauncherCmd;
use crate::widgets::ListRow;
use crate::widgets::base::PopupBase;
use crate::shell::ShellPopup;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct LauncherPopup {
    pub base: PopupBase,
    pub ctx: AppContext,
    pub entry: gtk4::Entry,
}

impl ShellPopup for LauncherPopup {
    fn id(&self) -> &str { "launcher" }
    fn is_open(&self) -> bool { *self.base.is_open.borrow() }

    fn close(&self) {
        self.base.close();
    }

    fn toggle(&self) {
        if self.is_open() {
            self.close();
        } else {
            self.on_open();
            self.base.open();
        }
    }
}

impl LauncherPopup {
    pub fn new(
        app: &libadwaita::Application,
        ctx: AppContext,
        on_state_change: impl Fn() + 'static,
    ) -> Self {
        let base = PopupBase::new(app, "Carp Launcher", false);
        
        let on_change = Rc::new(on_state_change);
        let on_change_c = on_change.clone();
        base.window.connect_visible_notify(move |_| {
            on_change_c();
        });

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

        // --- RIGHT PANE (Details) ---
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

        base.set_content(&container);

        // --- LOGIK ---
        let tx = ctx.launcher_tx.clone();
        let entry_c = entry.clone();
        entry.connect_changed(move |e| {
            let _ = tx.send_blocking(LauncherCmd::Search(e.text().to_string()));
        });

        // Navigation (Pfeiltasten)
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

        // Aktivierung (Enter)
        let tx_activate = ctx.launcher_tx.clone();
        let base_activate = base.clone();
        entry.connect_activate(move |_| {
            let _ = tx_activate.send_blocking(LauncherCmd::Activate(None)); // None = Nutze Selektion
            base_activate.close();
        });

        // Results Rendering
        let list_c = list.clone();
        let d_title = detail_title.clone();
        let d_desc = detail_desc.clone();
        let d_rev = detail_revealer.clone();
        let win_c = base.window.clone();
        let tx_row = ctx.launcher_tx.clone();
        let base_row = base.clone();

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
                
                let tx_inner = tx_row.clone();
                let base_inner = base_row.clone();
                let idx = i; // Index für den Closure capturen
                
                row.button.connect_clicked(move |_| {
                    // WICHTIG: Hier schicken wir den exakten Index des geklickten Items!
                    let _ = tx_inner.send_blocking(LauncherCmd::Activate(Some(idx)));
                    base_inner.close();
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

        Self { 
            ctx,
            base,
            entry: entry_c,
        }
    }

    fn on_open(&self) {
        self.entry.set_text("");
        self.entry.grab_focus();
        let _ = self.ctx.launcher_tx.send_blocking(LauncherCmd::Search("".to_string()));
    }
}

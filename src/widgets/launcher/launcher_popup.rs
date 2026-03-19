use crate::app_context::AppContext;
use crate::services::launcher::{LauncherCmd, LauncherUpdate};
use crate::shell::ShellPopup;
use crate::widgets::base::PopupBase;
use crate::widgets::launcher::launcher_row::LauncherRow;
use gtk4::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

pub struct LauncherPopup {
    pub base: PopupBase,
    pub ctx: AppContext,
    pub entry: gtk4::Entry,
}

impl ShellPopup for LauncherPopup {
    fn id(&self) -> &str {
        "launcher"
    }
    fn is_open(&self) -> bool {
        *self.base.is_open.borrow()
    }
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
        let base = PopupBase::new(app, "AXIS Launcher", false);

        let on_change = Rc::new(on_state_change);
        let on_change_c = on_change.clone();
        base.window.connect_visible_notify(move |_| on_change_c());

        // --- Layout ---
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.add_css_class("qs-panel");
        container.set_width_request(380);
        container.set_height_request(450);

        let left_pane = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        left_pane.set_width_request(380);
        left_pane.set_hexpand(true);

        let entry_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        entry_box.set_margin_bottom(12);
        let entry = gtk4::Entry::builder()
            .placeholder_text("Suchen, Finden, Machen...")
            .hexpand(true)
            .css_classes(vec!["qs-entry"])
            .build();
        entry_box.append(&entry);
        left_pane.append(&entry_box);

        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list", "launcher-list"])
            .selection_mode(gtk4::SelectionMode::Single)
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

        // --- Detail pane ---
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
            .css_classes(vec!["qs-subpage-title"])
            .wrap(true)
            .build();
        let detail_desc = gtk4::Label::builder()
            .halign(gtk4::Align::Start)
            .wrap(true)
            .css_classes(vec!["list-sublabel"])
            .build();
        detail_box.append(&detail_title);
        detail_box.append(&detail_desc);
        detail_revealer.set_child(Some(&detail_box));
        container.append(&detail_revealer);

        base.set_content(&container);

        // --- Search ---
        let tx = ctx.launcher_tx.clone();
        entry.connect_changed(move |e| {
            let _ = tx.send_blocking(LauncherCmd::Search(e.text().to_string()));
        });

        // --- Keyboard navigation ---
        let tx_key = ctx.launcher_tx.clone();
        let base_key = base.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, state| {
            use gtk4::gdk::Key;
            use gtk4::gdk::ModifierType;
            match key {
                Key::Down => {
                    let _ = tx_key.send_blocking(LauncherCmd::SelectNext);
                    gtk4::glib::Propagation::Stop
                }
                Key::Up => {
                    let _ = tx_key.send_blocking(LauncherCmd::SelectPrev);
                    gtk4::glib::Propagation::Stop
                }
                Key::Tab => {
                    if state.contains(ModifierType::SHIFT_MASK) {
                        let _ = tx_key.send_blocking(LauncherCmd::SelectPrev);
                    } else {
                        let _ = tx_key.send_blocking(LauncherCmd::SelectNext);
                    }
                    gtk4::glib::Propagation::Stop
                }
                Key::Escape => {
                    base_key.close();
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });
        entry.add_controller(key_controller);

        // --- Activation (Enter) ---
        let tx_activate = ctx.launcher_tx.clone();
        let base_activate = base.clone();
        entry.connect_activate(move |_| {
            let _ = tx_activate.send_blocking(LauncherCmd::Activate(None));
            base_activate.close();
        });

        // --- State subscription ---
        let list_c = list.clone();
        let scrolled_c = scrolled.clone();
        let d_title = detail_title.clone();
        let d_desc = detail_desc.clone();
        let d_rev = detail_revealer.clone();
        let win_c = base.window.clone();
        let tx_row = ctx.launcher_tx.clone();
        let base_row = base.clone();

        ctx.launcher.subscribe(move |data| {
            match data.update_kind {
                // Full rebuild: new search results arrived.
                LauncherUpdate::Results => {
                    while let Some(child) = list_c.first_child() {
                        list_c.remove(&child);
                    }

                    for (i, item) in data.results.iter().enumerate() {
                        let launcher_row = LauncherRow::new(item);
                        let tx_inner = tx_row.clone();
                        let base_inner = base_row.clone();

                        launcher_row.row.connect_activate(move |_| {
                            let _ = tx_inner.send_blocking(LauncherCmd::Activate(Some(i)));
                            base_inner.close();
                        });

                        list_c.append(&launcher_row.row);
                    }
                }

                // Selection only: update the highlighted row without rebuilding.
                LauncherUpdate::SelectionOnly => {}
            }

            // Sync GTK selection + detail pane regardless of update kind.
            if data.results.is_empty() {
                list_c.unselect_all();
                d_rev.set_reveal_child(false);
                win_c.set_width_request(380);
            } else if let Some(idx) = data.selected_index {
                if let Some(row) = list_c.row_at_index(idx as i32) {
                    list_c.select_row(Some(&row));

                    // Scroll so the selected row is centered in the viewport.
                    // translate_coordinates() gives the real Y offset of the row
                    // within the list — no hardcoded heights needed.
                    let adj = scrolled_c.vadjustment();
                    if let Some((_, row_y)) = row.translate_coordinates(&list_c, 0.0, 0.0) {
                        let row_h = row.height() as f64;
                        let page = adj.page_size();
                        let target =
                            (row_y - (page / 2.0) + (row_h / 2.0)).clamp(0.0, adj.upper() - page);
                        smooth_scroll(&adj, target);
                    }

                    if let Some(item) = data.results.get(idx) {
                        d_title.set_text(&item.title);
                        d_desc.set_text(item.description.as_deref().unwrap_or(""));
                        d_rev.set_reveal_child(true);
                        win_c.set_width_request(380 + 300);
                    }
                }
            }
        });

        let entry_c = entry.clone();
        Self {
            ctx,
            base,
            entry: entry_c,
        }
    }

    fn on_open(&self) {
        self.entry.set_text("");
        self.entry.grab_focus();
        let _ = self
            .ctx
            .launcher_tx
            .send_blocking(LauncherCmd::Search("".to_string()));
    }
}

/// Animates a `gtk4::Adjustment` from its current value to `target` using
/// an ease-out curve over ~120 ms (8 frames @ 60 Hz).
fn smooth_scroll(adj: &gtk4::Adjustment, target: f64) {
    const STEPS: u32 = 8;
    const INTERVAL: Duration = Duration::from_millis(15);

    let start = adj.value();
    let delta = target - start;

    // Skip animation for tiny movements — avoids jitter on first item.
    if delta.abs() < 1.0 {
        return;
    }

    let adj = adj.clone();
    let step = Rc::new(Cell::new(0u32));

    gtk4::glib::timeout_add_local(INTERVAL, move || {
        let s = step.get() + 1;
        step.set(s);

        // Ease-out: t starts fast and decelerates.
        let t = s as f64 / STEPS as f64;
        let ease = 1.0 - (1.0 - t).powi(3);
        adj.set_value(start + delta * ease);

        if s >= STEPS {
            gtk4::glib::ControlFlow::Break
        } else {
            gtk4::glib::ControlFlow::Continue
        }
    });
}

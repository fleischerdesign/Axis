use crate::app_context::AppContext;
use crate::services::launcher::{LauncherCmd, LauncherUpdate};
use crate::shell::PopupExt;
use crate::widgets::base::PopupBase;
use crate::widgets::launcher::launcher_row::LauncherRow;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

struct RowEntry {
    launcher_row: LauncherRow,
    list_box_row: gtk4::ListBoxRow,
}

pub struct LauncherPopup {
    pub base: PopupBase,
    pub ctx: AppContext,
    pub entry: gtk4::Entry,
}

impl PopupExt for LauncherPopup {
    fn id(&self) -> &str {
        "launcher"
    }

    fn base(&self) -> &PopupBase {
        &self.base
    }

    fn on_open(&self) {
        self.entry.set_text("");
        self.entry.grab_focus();
        let _ = self
            .ctx
            .launcher
            .tx
            .try_send(LauncherCmd::Search("".to_string()));
    }
}

impl LauncherPopup {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Self {
        let base = PopupBase::new(app, "AXIS Launcher", false);

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

        let tx_click = ctx.launcher.tx.clone();
        let base_click = base.clone();
        list.connect_row_activated(move |_, row| {
            let idx = row.index() as usize;
            let _ = tx_click.try_send(LauncherCmd::Activate(Some(idx)));
            base_click.close();
        });

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
            .css_classes(vec!["subpage-title"])
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
        let tx = ctx.launcher.tx.clone();
        entry.connect_changed(move |e| {
            let _ = tx.try_send(LauncherCmd::Search(e.text().to_string()));
        });

        // --- Keyboard navigation (Escape on entry-level, register() handles window-level) ---
        let tx_key = ctx.launcher.tx.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, state| {
            use gtk4::gdk::Key;
            use gtk4::gdk::ModifierType;
            match key {
                Key::Down => {
                    let _ = tx_key.try_send(LauncherCmd::SelectNext);
                    gtk4::glib::Propagation::Stop
                }
                Key::Up => {
                    let _ = tx_key.try_send(LauncherCmd::SelectPrev);
                    gtk4::glib::Propagation::Stop
                }
                Key::Tab => {
                    if state.contains(ModifierType::SHIFT_MASK) {
                        let _ = tx_key.try_send(LauncherCmd::SelectPrev);
                    } else {
                        let _ = tx_key.try_send(LauncherCmd::SelectNext);
                    }
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });
        entry.add_controller(key_controller);

        // --- Activation (Enter) ---
        let tx_activate = ctx.launcher.tx.clone();
        let base_activate = base.clone();
        entry.connect_activate(move |_| {
            let _ = tx_activate.try_send(LauncherCmd::Activate(None));
            base_activate.close();
        });

        // --- State subscription ---
        let list_c = list.clone();
        let scrolled_c = scrolled.clone();
        let d_title = detail_title.clone();
        let d_desc = detail_desc.clone();
        let d_rev = detail_revealer.clone();
        let win_c = base.window.clone();
        let rows: Rc<RefCell<HashMap<String, RowEntry>>> = Rc::new(RefCell::new(HashMap::new()));
        let prev_selected: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
        ctx.launcher.subscribe(move |data| {
            match data.update_kind {
                LauncherUpdate::Results => {
                    let mut rows = rows.borrow_mut();

                    let new_ids: std::collections::HashSet<&str> = data.results.iter().map(|r| r.id.as_str()).collect();

                    let stale: Vec<String> = rows
                        .keys()
                        .filter(|id| !new_ids.contains(id.as_str()))
                        .cloned()
                        .collect();
                    for id in stale {
                        if let Some(entry) = rows.remove(&id) {
                            list_c.remove(&entry.list_box_row);
                        }
                    }

                    for (idx, item) in data.results.iter().enumerate() {
                        if let Some(entry) = rows.get(&item.id) {
                            entry.launcher_row.update(item);
                            continue;
                        }

                        let launcher_row = LauncherRow::new(item);
                        let list_box_row = gtk4::ListBoxRow::builder()
                            .selectable(false)
                            .activatable(true)
                            .child(&launcher_row.row)
                            .build();

                        rows.insert(
                            item.id.clone(),
                            RowEntry { launcher_row, list_box_row: list_box_row.clone() },
                        );
                        list_c.insert(&list_box_row, idx as i32);
                    }
                }
                LauncherUpdate::SelectionOnly => {}
            }

            if data.results.is_empty() {
                list_c.unselect_all();
                d_rev.set_reveal_child(false);
                win_c.set_width_request(380);
                if let Some(prev) = prev_selected.borrow_mut().take() {
                    let rows = rows.borrow();
                    for entry in rows.values() {
                        if entry.list_box_row.index() as usize == prev {
                            entry.launcher_row.row.remove_css_class("launcher-selected");
                            break;
                        }
                    }
                }
            } else if let Some(idx) = data.selected_index {
                let mut prev = prev_selected.borrow_mut();
                if let Some(old_idx) = *prev {
                    let rows = rows.borrow();
                    for entry in rows.values() {
                        if entry.list_box_row.index() as usize == old_idx {
                            entry.launcher_row.row.remove_css_class("launcher-selected");
                            break;
                        }
                    }
                }

                if let Some(row) = list_c.row_at_index(idx as i32) {
                    list_c.select_row(Some(&row));

                    let rows = rows.borrow();
                    for entry in rows.values() {
                        if entry.list_box_row.index() == row.index() {
                            entry.launcher_row.row.add_css_class("launcher-selected");
                            break;
                        }
                    }
                    drop(rows);

                    let adj = scrolled_c.vadjustment();
                    if let Some(point) =
                        row.compute_point(&list_c, &gtk4::graphene::Point::new(0.0, 0.0))
                    {
                        let row_y = point.y() as f64;
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

                *prev = Some(idx);
            }
        });

        let entry_c = entry.clone();
        Self {
            ctx,
            base,
            entry: entry_c,
        }
    }
}

fn smooth_scroll(adj: &gtk4::Adjustment, target: f64) {
    const STEPS: u32 = 8;
    const INTERVAL: Duration = Duration::from_millis(15);

    let start = adj.value();
    let delta = target - start;

    if delta.abs() < 1.0 {
        return;
    }

    let adj = adj.clone();
    let step = Rc::new(Cell::new(0u32));

    gtk4::glib::timeout_add_local(INTERVAL, move || {
        let s = step.get() + 1;
        step.set(s);

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

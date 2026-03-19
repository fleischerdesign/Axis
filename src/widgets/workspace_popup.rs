use crate::app_context::AppContext;
use crate::services::niri::NiriService;
use crate::shell::ShellPopup;
use crate::widgets::base::PopupBase;
use gtk4::prelude::*;
use gtk4_layer_shell::LayerShell;
use std::cell::Cell;
use std::rc::Rc;

pub struct WorkspacePopup {
    pub base: PopupBase,
    ctx: AppContext,
    focused_index: Rc<Cell<usize>>,
}

impl ShellPopup for WorkspacePopup {
    fn id(&self) -> &str {
        "ws"
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

impl WorkspacePopup {
    pub fn new(
        app: &libadwaita::Application,
        ctx: AppContext,
        on_state_change: impl Fn() + 'static,
    ) -> Self {
        let base = PopupBase::new(app, "AXIS Workspace Popup", false);
        let on_state_change = Rc::new(on_state_change);
        let focused_index = Rc::new(Cell::new(0));

        // State-Change an den Controller melden
        let on_change_c = on_state_change.clone();
        base.window.connect_visible_notify(move |_| {
            on_change_c();
        });

        // Den Workspace-Shelf mittig ausrichten (keine Anker für Links/Rechts = Zentriert)
        base.window.set_anchor(gtk4_layer_shell::Edge::Left, false);
        base.window.set_anchor(gtk4_layer_shell::Edge::Right, false);

        let shelf = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
        shelf.add_css_class("workspace-shelf");
        base.set_content(&shelf);

        // --- KEYBOARD NAVIGATION ---
        let base_close = base.clone();
        let shelf_kb = shelf.clone();
        let focused_index_kb = focused_index.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                gtk4::gdk::Key::Escape => {
                    base_close.close();
                    return gtk4::glib::Propagation::Stop;
                }
                gtk4::gdk::Key::Right | gtk4::gdk::Key::Tab => {
                    let count = Self::child_count(&shelf_kb);
                    if count > 0 {
                        let next = (focused_index_kb.get() + 1) % count;
                        focused_index_kb.set(next);
                        Self::focus_child_at(&shelf_kb, next);
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                gtk4::gdk::Key::Left => {
                    let count = Self::child_count(&shelf_kb);
                    if count > 0 {
                        let prev = if focused_index_kb.get() == 0 {
                            count - 1
                        } else {
                            focused_index_kb.get() - 1
                        };
                        focused_index_kb.set(prev);
                        Self::focus_child_at(&shelf_kb, prev);
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter => {
                    // Activate the currently focused child
                    if let Some(child) = Self::child_at(&shelf_kb, focused_index_kb.get()) {
                        if let Some(btn) = child.downcast_ref::<gtk4::Button>() {
                            btn.emit_clicked();
                        }
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                _ => {}
            }
            gtk4::glib::Propagation::Proceed
        });
        base.window.add_controller(key_controller);

        let shelf_c = shelf.clone();
        let window_c = base.window.clone();
        let is_open_c = base.is_open.clone();
        let close_popup = Rc::new({
            let base = base.clone();
            move || base.close()
        });
        let close_popup_c = close_popup.clone();
        ctx.niri.subscribe(move |data| {
            if *is_open_c.borrow() {
                Self::render_shelf(&shelf_c, data, &window_c, close_popup_c.clone());
            }
        });

        Self {
            base,
            ctx,
            focused_index,
        }
    }

    fn on_open(&self) {
        self.focused_index.set(0);
        if let Some(shelf) = self
            .base
            .revealer
            .child()
            .and_then(|c| c.downcast::<gtk4::Box>().ok())
        {
            let close_popup = Rc::new({
                let base = self.base.clone();
                move || base.close()
            });
            Self::render_shelf(&shelf, &self.ctx.niri.get(), &self.base.window, close_popup);
            // Focus first workspace card on open
            gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {});
        }
    }

    fn child_count(shelf: &gtk4::Box) -> usize {
        let mut count = 0;
        let mut child = shelf.first_child();
        while child.is_some() {
            count += 1;
            child = child.and_then(|c| c.next_sibling());
        }
        count
    }

    fn child_at(shelf: &gtk4::Box, index: usize) -> Option<gtk4::Widget> {
        let mut current = shelf.first_child()?;
        for _ in 0..index {
            current = current.next_sibling()?;
        }
        Some(current)
    }

    fn focus_child_at(shelf: &gtk4::Box, index: usize) {
        if let Some(child) = Self::child_at(shelf, index) {
            child.grab_focus();
        }
    }

    fn render_shelf(
        shelf: &gtk4::Box,
        data: &crate::services::niri::NiriData,
        window: &gtk4::Window,
        close_popup: Rc<dyn Fn()>,
    ) {
        while let Some(child) = shelf.first_child() {
            shelf.remove(&child);
        }
        let mut workspaces = data.workspaces.clone();
        workspaces.sort_by_key(|w| w.id);
        let mut windows = data.windows.clone();
        windows.sort_by_key(|w| w.layout.pos_in_scrolling_layout.unwrap_or((0, 0)));

        for ws in workspaces {
            let (m_w, m_h) = data
                .outputs
                .get(ws.output.as_deref().unwrap_or(""))
                .and_then(|o| o.logical.as_ref())
                .map(|l| (l.width as f64, l.height as f64))
                .unwrap_or((1920.0, 1080.0));

            let card_w = 220.0;
            let margin = 15.0;
            let card_h = ((card_w - margin * 2.0) / (m_w / m_h)) + margin * 2.0;

            let card = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .width_request(card_w as i32)
                .css_classes(vec!["workspace-card".to_string()])
                .build();
            if ws.is_active {
                card.add_css_class("active");
            }

            let preview_scroll = gtk4::ScrolledWindow::builder()
                .width_request(card_w as i32)
                .height_request(card_h as i32)
                .hscrollbar_policy(gtk4::PolicyType::Never)
                .vscrollbar_policy(gtk4::PolicyType::Never)
                .css_classes(vec!["workspace-preview".to_string()])
                .build();

            let preview_inner = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            preview_inner.set_halign(gtk4::Align::Center);
            preview_inner.set_valign(gtk4::Align::Center);
            preview_inner.set_margin_start(margin as i32);
            preview_inner.set_margin_end(margin as i32);
            preview_inner.set_margin_top(margin as i32);
            preview_inner.set_margin_bottom(margin as i32);
            preview_scroll.set_child(Some(&preview_inner));

            let scale = (card_w - margin * 2.0) / m_w;
            let mut cur_col: Option<gtk4::Box> = None;
            let mut last_col_idx = None;

            for win in &windows {
                if win.workspace_id != Some(ws.id) {
                    continue;
                }
                let (w_px, h_px) = win.layout.tile_size;
                let col_idx = win.layout.pos_in_scrolling_layout.map(|p| p.0).unwrap_or(0);

                if Some(col_idx) != last_col_idx || cur_col.is_none() {
                    let col = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
                    col.set_valign(gtk4::Align::Center);
                    preview_inner.append(&col);
                    cur_col = Some(col);
                    last_col_idx = Some(col_idx);
                }

                if let Some(col) = &cur_col {
                    let win_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                    win_box.set_size_request((w_px * scale) as i32, (h_px * scale) as i32);
                    win_box.add_css_class("preview-window");
                    if win.is_focused {
                        win_box.add_css_class("focused");
                    }

                    let app_icon = gtk4::Image::from_icon_name(
                        win.app_id.as_deref().unwrap_or("application-x-executable"),
                    );
                    app_icon.set_pixel_size(((h_px * scale) / 2.0) as i32);
                    app_icon.set_halign(gtk4::Align::Center);
                    app_icon.set_valign(gtk4::Align::Center);
                    app_icon.set_hexpand(true);
                    app_icon.set_vexpand(true);
                    win_box.append(&app_icon);
                    col.append(&win_box);
                }
            }

            card.append(&preview_scroll);
            card.append(&gtk4::Label::new(Some(&format!("Workspace {}", ws.id))));

            let ws_id = ws.id;
            let close_popup_ws = close_popup.clone();
            let btn = gtk4::Button::new();
            btn.add_css_class("workspace-card");
            btn.set_child(Some(&card));
            btn.connect_clicked(move |_| {
                NiriService::switch_to_workspace(ws_id);
                close_popup_ws();
            });

            shelf.append(&btn);
        }

        if window.is_visible() {
            window.set_default_size(1, 1);
        }
    }
}

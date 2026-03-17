use crate::app_context::AppContext;
use crate::shell::ShellPopup;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

pub struct WorkspacePopup {
    pub window: gtk4::Window,
    pub is_open: Rc<RefCell<bool>>,
    ctx: AppContext,
    on_state_change: Rc<dyn Fn() + 'static>,
}

impl ShellPopup for WorkspacePopup {
    fn id(&self) -> &str { "ws" }
    fn is_open(&self) -> bool { *self.is_open.borrow() }

    fn close(&self) {
        if !self.is_open() { return; }
        *self.is_open.borrow_mut() = false;
        self.animate_close();
        (self.on_state_change)();
    }

    fn toggle(&self) {
        if self.is_open() {
            self.close();
        } else {
            *self.is_open.borrow_mut() = true;
            self.animate_open();
            (self.on_state_change)();
        }
    }
}

impl WorkspacePopup {
    pub fn new(app: &libadwaita::Application, ctx: AppContext, on_state_change: impl Fn() + 'static) -> Self {
        let is_open = Rc::new(RefCell::new(false));
        let on_state_change = Rc::new(on_state_change);

        let window = gtk4::Window::builder()
            .application(app)
            .title("Carp Workspace Popup")
            .visible(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Bottom, true);
        window.set_margin(Edge::Bottom, 64);

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();
        let shelf = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
        shelf.add_css_class("workspace-shelf");
        revealer.set_child(Some(&shelf));
        window.set_child(Some(&revealer));

        let shelf_c = shelf.clone();
        let window_c = window.clone();
        let is_open_c = is_open.clone();
        ctx.niri.subscribe(move |data| {
            if *is_open_c.borrow() {
                Self::render_shelf(&shelf_c, data, &window_c);
            }
        });

        Self { window, is_open, ctx, on_state_change }
    }

    fn animate_open(&self) {
        let revealer = self.window.child().and_then(|c| c.downcast::<gtk4::Revealer>().ok()).unwrap();
        let shelf = revealer.child().and_then(|c| c.downcast::<gtk4::Box>().ok()).unwrap();
        
        Self::render_shelf(&shelf, &self.ctx.niri.get(), &self.window);
        self.window.set_visible(true);
        revealer.set_reveal_child(true);
    }

    fn animate_close(&self) {
        if let Some(rev) = self.window.child().and_then(|c| c.downcast::<gtk4::Revealer>().ok()) {
            rev.set_reveal_child(false);
            let win = self.window.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(280), move || {
                win.set_visible(false);
                gtk4::glib::ControlFlow::Break
            });
        }
    }

    fn render_shelf(shelf: &gtk4::Box, data: &crate::services::niri::NiriData, window: &gtk4::Window) {
        while let Some(child) = shelf.first_child() { shelf.remove(&child); }
        let mut workspaces = data.workspaces.clone();
        workspaces.sort_by_key(|w| w.id);
        let mut windows = data.windows.clone();
        windows.sort_by_key(|w| w.layout.pos_in_scrolling_layout.unwrap_or((0, 0)));

        for ws in workspaces {
            let (m_w, m_h) = data.outputs.get(ws.output.as_deref().unwrap_or(""))
                .and_then(|o| o.logical.as_ref())
                .map(|l| (l.width as f64, l.height as f64))
                .unwrap_or((1920.0, 1080.0));

            let card_w = 220.0;
            let margin = 15.0;
            let card_h = ((card_w - margin * 2.0) / (m_w / m_h)) + margin * 2.0;

            let card = gtk4::Box::builder().orientation(gtk4::Orientation::Vertical)
                .width_request(card_w as i32).css_classes(vec!["workspace-card".to_string()]).build();
            if ws.is_active { card.add_css_class("active"); }

            let preview_scroll = gtk4::ScrolledWindow::builder()
                .width_request(card_w as i32).height_request(card_h as i32)
                .hscrollbar_policy(gtk4::PolicyType::Never).vscrollbar_policy(gtk4::PolicyType::Never)
                .css_classes(vec!["workspace-preview".to_string()]).build();

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
                if win.workspace_id != Some(ws.id) { continue; }
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
                    if win.is_focused { win_box.add_css_class("focused"); }

                    let app_icon = gtk4::Image::from_icon_name(win.app_id.as_deref().unwrap_or("application-x-executable"));
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
            shelf.append(&card);
        }

        if window.is_visible() { window.set_default_size(1, 1); }
    }
}

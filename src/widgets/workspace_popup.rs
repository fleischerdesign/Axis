use gtk4::prelude::*;
use gtk4_layer_shell::{Layer, Edge, LayerShell};
use crate::services::niri::NiriService;
use std::rc::Rc;
use std::cell::RefCell;
use futures_util::StreamExt;
use chrono::Local;

pub struct WorkspacePopup {
    pub window: gtk4::Window,
    pub is_open: Rc<RefCell<bool>>,
}

impl WorkspacePopup {
    pub fn new(app: &libadwaita::Application, clock_label: &gtk4::Label, ws_label: &gtk4::Label) -> Self {
        let is_open = Rc::new(RefCell::new(false));

        let window = gtk4::Window::builder()
            .application(app)
            .title("Carp Workspace Popup")
            .visible(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Bottom, true);
        window.set_margin(Edge::Bottom, 10);

        let ws_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();
        let shelf_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
        shelf_box.add_css_class("workspace-shelf");
        ws_revealer.set_child(Some(&shelf_box));
        window.set_child(Some(&ws_revealer));

        let mut niri_rx = NiriService::spawn();
        let ws_label_c = ws_label.clone();
        let clock_label_c = clock_label.clone();
        let shelf_box_c = shelf_box.clone();
        let ws_popup_c = window.clone();

        gtk4::glib::MainContext::default().spawn_local(async move {
            while let Some(data) = niri_rx.next().await {
                let mut workspaces = data.workspaces;
                workspaces.sort_by_key(|w| w.id);
                let mut windows = data.windows;
                windows.sort_by_key(|w| w.layout.pos_in_scrolling_layout.unwrap_or((0, 0)));
                
                clock_label_c.set_text(&Local::now().format("%H:%M").to_string());
                
                let mut ws_markup = String::new();
                for ws in &workspaces {
                    if ws.is_active { ws_markup.push_str(&format!(" <b>{}</b> ", ws.id)); }
                    else { ws_markup.push_str(&format!(" {} ", ws.id)); }
                }
                ws_label_c.set_markup(&ws_markup);

                while let Some(child) = shelf_box_c.first_child() { shelf_box_c.remove(&child); }
                
                for ws in workspaces {
                    let (m_w, m_h) = if let Some(o) = data.outputs.get(ws.output.as_deref().unwrap_or("")) {
                        if let Some(l) = &o.logical { (l.width as f64, l.height as f64) }
                        else { (1920.0, 1080.0) }
                    } else { (1920.0, 1080.0) };

                    let cw = 220.0;
                    let m = 15.0;
                    let ch = ((cw - m * 2.0) / (m_w / m_h)) + m * 2.0;

                    let card = gtk4::Box::builder().orientation(gtk4::Orientation::Vertical).width_request(cw as i32).css_classes(vec!["workspace-card".to_string()]).build();
                    if ws.is_active { card.add_css_class("active"); }

                    let sc = gtk4::ScrolledWindow::builder().width_request(cw as i32).height_request(ch as i32).hscrollbar_policy(gtk4::PolicyType::Never).vscrollbar_policy(gtk4::PolicyType::Never).css_classes(vec!["workspace-preview".to_string()]).build();
                    let st = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                    st.set_halign(gtk4::Align::Center); st.set_valign(gtk4::Align::Center);
                    st.set_margin_start(m as i32); st.set_margin_end(m as i32); st.set_margin_top(m as i32); st.set_margin_bottom(m as i32);
                    sc.set_child(Some(&st));

                    let scale = (cw - m * 2.0) / m_w;
                    let mut cur_col: Option<gtk4::Box> = None;
                    let mut last_idx = None;

                    for win in &windows {
                        if win.workspace_id == Some(ws.id) {
                            let (w_r, h_r) = win.layout.tile_size;
                            let c_idx = win.layout.pos_in_scrolling_layout.map(|p| p.0).unwrap_or(0);
                            if Some(c_idx) != last_idx || cur_col.is_none() {
                                let nc = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
                                nc.set_valign(gtk4::Align::Center);
                                st.append(&nc);
                                cur_col = Some(nc);
                                last_idx = Some(c_idx);
                            }
                            if let Some(cb) = &cur_col {
                                let wb = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                                wb.set_size_request((w_r * scale) as i32, (h_r * scale) as i32);
                                wb.add_css_class("preview-window");
                                if win.is_focused { wb.add_css_class("focused"); }
                                let ic = gtk4::Image::from_icon_name(win.app_id.as_deref().unwrap_or("application-x-executable"));
                                ic.set_pixel_size(((h_r * scale) / 2.0) as i32);
                                ic.set_halign(gtk4::Align::Center); ic.set_valign(gtk4::Align::Center);
                                ic.set_hexpand(true); ic.set_vexpand(true);
                                wb.append(&ic);
                                cb.append(&wb);
                            }
                        }
                    }
                    card.append(&sc);
                    card.append(&gtk4::Label::new(Some(&format!("Workspace {}", ws.id))));
                    shelf_box_c.append(&card);
                }
                if ws_popup_c.is_visible() { ws_popup_c.set_default_size(1, 1); }
            }
        });

        Self { window, is_open }
    }

    pub fn toggle(&self) {
        let mut open = self.is_open.borrow_mut();
        *open = !*open;
        let revealer = self.window.child().and_then(|c| c.downcast::<gtk4::Revealer>().ok()).unwrap();
        if *open {
            self.window.set_visible(true);
            revealer.set_reveal_child(true);
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

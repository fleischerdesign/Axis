use crate::app_context::AppContext;
use crate::services::tray::{TrayCmd, TrayData, TrayItem};
use crate::widgets::Island;
use gtk4::prelude::*;
use gtk4::GestureClick;
use log::debug;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct BarTray {
    pub container: gtk4::Box,
}

impl BarTray {
    pub fn new(ctx: AppContext) -> Self {
        let island = Island::new(8);
        island.container.set_cursor_from_name(Some("pointer"));

        let icons: Rc<RefCell<HashMap<String, gtk4::Image>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let island_c = island.container.clone();
        let icons_c = icons.clone();
        let ctx_c = ctx.clone();

        ctx.tray.subscribe(move |data: &TrayData| {
            island_c.set_visible(!data.items.is_empty());

            let mut current = icons_c.borrow_mut();

            // Remove icons for items that no longer exist
            let active_bus_names: Vec<String> =
                data.items.iter().map(|i| i.bus_name.clone()).collect();
            let stale: Vec<String> = current
                .keys()
                .filter(|k| !active_bus_names.contains(k))
                .cloned()
                .collect();
            for key in stale {
                if let Some(img) = current.remove(&key) {
                    island_c.remove(&img);
                }
            }

            // Add or update icons
            for item in &data.items {
                if let Some(img) = current.get(&item.bus_name) {
                    // Update icon
                    let icon = Self::pick_icon(item);
                    img.set_icon_name(Some(icon));
                } else {
                    // New icon
                    let icon = Self::pick_icon(item);
                    let img = gtk4::Image::from_icon_name(icon);
                    img.set_pixel_size(16);

                    // Left click → Activate
                    let click = GestureClick::new();
                    click.set_button(0);
                    let bn = item.bus_name.clone();
                    let ctx_click = ctx_c.clone();
                    click.connect_pressed(move |gesture, _, _, _| match gesture.current_button() {
                        1 => {
                            debug!("[tray] Left click: {bn}");
                            let _ = ctx_click
                                .tray_tx
                                .send_blocking(TrayCmd::SecondaryActivate(bn.clone()));
                        }
                        3 => {
                            debug!("[tray] Right click: {bn}");
                            let _ = ctx_click
                                .tray_tx
                                .send_blocking(TrayCmd::ContextMenu(bn.clone()));
                        }
                        2 => {
                            let _ = ctx_click
                                .tray_tx
                                .send_blocking(TrayCmd::SecondaryActivate(bn.clone()));
                        }
                        _ => {}
                    });
                    img.add_controller(click);

                    // Tooltip
                    if !item.title.is_empty() {
                        img.set_tooltip_text(Some(&item.title));
                    }

                    island_c.append(&img);
                    current.insert(item.bus_name.clone(), img);
                }
            }
        });

        Self {
            container: island.container,
        }
    }

    fn pick_icon(item: &TrayItem) -> &str {
        if item.status == "NeedsAttention" && !item.attention_icon_name.is_empty() {
            return &item.attention_icon_name;
        }
        if !item.icon_name.is_empty() {
            return &item.icon_name;
        }
        "application-x-executable-symbolic"
    }
}

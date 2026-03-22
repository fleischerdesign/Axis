use crate::app_context::AppContext;
use crate::services::niri::NiriData;
use crate::widgets::Island;
use gtk4::prelude::*;

pub struct BarCenter {
    pub container: gtk4::Box,
    pub ws_container: gtk4::Box,
}

impl BarCenter {
    pub fn new(ctx: AppContext) -> Self {
        let island = Island::new(12);
        island.container.set_cursor_from_name(Some("pointer"));

        let ws_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        ws_container.add_css_class("workspace-dots");

        let ws_container_clone = ws_container.clone();
        let ws_container_for_ui = ws_container.clone();

        let clock_label = gtk4::Label::new(None);
        clock_label.add_css_class("clock-label");

        island.append(&ws_container);
        island.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        island.append(&clock_label);

        // Subscriptions
        ctx.clock.subscribe(move |time| {
            clock_label.set_text(&time.format("%H:%M").to_string());
        });

        ctx.niri.subscribe(move |data| {
            Self::update_workspaces(&ws_container_clone, data);
        });

        Self {
            container: island.container,
            ws_container: ws_container_for_ui,
        }
    }

    fn update_workspaces(container: &gtk4::Box, data: &NiriData) {
        let mut workspaces = data.workspaces.clone();
        workspaces.sort_by_key(|w| w.id);

        let target = workspaces.len();

        // Remove excess dots
        while Self::child_count(container) > target {
            if let Some(last) = container.last_child() {
                container.remove(&last);
            }
        }

        // Add missing dots or update existing
        let mut child = container.first_child();
        for ws in workspaces.iter() {
            if let Some(dot) = child {
                // Update existing dot
                if ws.is_active {
                    dot.add_css_class("active");
                } else {
                    dot.remove_css_class("active");
                }
                child = dot.next_sibling();
            } else {
                // Create new dot
                let dot = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
                dot.add_css_class("ws-dot");
                dot.set_hexpand(false);
                dot.set_vexpand(false);
                dot.set_valign(gtk4::Align::Center);
                if ws.is_active {
                    dot.add_css_class("active");
                }
                container.append(&dot);
            }
        }
    }

    fn child_count(container: &gtk4::Box) -> usize {
        let mut count = 0;
        let mut child = container.first_child();
        while child.is_some() {
            count += 1;
            child = child.and_then(|c| c.next_sibling());
        }
        count
    }
}

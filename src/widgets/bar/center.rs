use gtk4::prelude::*;
use crate::widgets::Island;
use crate::app_context::AppContext;
use crate::services::niri::NiriData;

pub struct BarCenter {
    pub container: gtk4::Box,
}

impl BarCenter {
    pub fn new(ctx: AppContext) -> Self {
        let island = Island::new(12);
        island.container.set_cursor_from_name(Some("pointer"));

        let ws_label = gtk4::Label::new(None);
        ws_label.add_css_class("workspace-label");
        
        let clock_label = gtk4::Label::new(None);
        clock_label.add_css_class("clock-label");

        island.append(&ws_label);
        island.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        island.append(&clock_label);

        // Subscriptions
        ctx.clock.subscribe(move |time| {
            clock_label.set_text(&time.format("%H:%M").to_string());
        });

        ctx.niri.subscribe(move |data| {
            Self::update_workspaces(&ws_label, data);
        });

        Self {
            container: island.container,
        }
    }

    fn update_workspaces(label: &gtk4::Label, data: &NiriData) {
        let mut workspaces = data.workspaces.clone();
        workspaces.sort_by_key(|w| w.id);
        let mut markup = String::new();
        for ws in workspaces {
            if ws.is_active {
                markup.push_str(&format!(" <b>{}</b> ", ws.id));
            } else {
                markup.push_str(&format!(" {} ", ws.id));
            }
        }
        label.set_markup(&markup);
    }
}

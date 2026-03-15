use gtk4::prelude::*;
use gtk4_layer_shell::{Layer, Edge, LayerShell};
use crate::widgets::Island;

pub struct Bar {
    pub window: gtk4::ApplicationWindow,
    pub ws_label: gtk4::Label,
    pub clock_label: gtk4::Label,
    pub vol_icon: gtk4::Image,
    pub status_island: gtk4::Box,
    pub center_island: gtk4::Box,
}

impl Bar {
    pub fn new(app: &libadwaita::Application) -> Self {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Carp Bottom Bar")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_exclusive_zone(54);

        let root = gtk4::CenterBox::new();
        root.set_margin_bottom(10);
        root.set_height_request(44);

        // --- 1. Launcher ---
        let launcher_island = Island::new(0);
        launcher_island.append(&gtk4::Image::from_icon_name("view-app-grid-symbolic"));
        root.set_start_widget(Some(&launcher_island.container));

        // --- 2. Center (Workspace & Clock) ---
        let center_island = Island::new(12);
        center_island.container.set_cursor_from_name(Some("pointer"));
        let ws_label = gtk4::Label::new(None);
        ws_label.add_css_class("workspace-label");
        let clock_label = gtk4::Label::new(None);
        clock_label.add_css_class("clock-label");
        center_island.append(&ws_label);
        center_island.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        center_island.append(&clock_label);
        root.set_center_widget(Some(&center_island.container));

        // --- 3. Status ---
        let status_island = Island::new(12);
        status_island.container.set_cursor_from_name(Some("pointer"));
        status_island.append(&gtk4::Image::from_icon_name("network-wireless-signal-excellent-symbolic"));
        let vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        status_island.append(&vol_icon);
        status_island.append(&gtk4::Image::from_icon_name("battery-full-symbolic"));
        root.set_end_widget(Some(&status_island.container));

        window.set_child(Some(&root));

        Self {
            window,
            ws_label,
            clock_label,
            vol_icon,
            status_island: status_island.container,
            center_island: center_island.container,
        }
    }
}

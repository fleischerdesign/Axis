use gtk4::prelude::*;
use gtk4_layer_shell::{Layer, Edge, LayerShell};
use crate::widgets::Island;
use crate::app_context::AppContext;

pub struct Bar {
    pub window: gtk4::ApplicationWindow,
    pub ws_label: gtk4::Label,
    pub clock_label: gtk4::Label,
    pub vol_icon: gtk4::Image,
    pub status_island: gtk4::Box,
    pub center_island: gtk4::Box,
}

impl Bar {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Self {
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
        
        let wifi_icon = gtk4::Image::from_icon_name("network-wireless-signal-excellent-symbolic");
        status_island.append(&wifi_icon);
        
        let vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        status_island.append(&vol_icon);
        
        let battery_icon = gtk4::Image::from_icon_name("battery-full-symbolic");
        status_island.append(&battery_icon);
        
        root.set_end_widget(Some(&status_island.container));
        window.set_child(Some(&root));

        // --- EVENTS ---
        let wifi_icon_c = wifi_icon.clone();
        let mut network_rx = ctx.network_rx.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            Self::update_wifi(&wifi_icon_c, &network_rx.borrow());
            while network_rx.changed().await.is_ok() {
                Self::update_wifi(&wifi_icon_c, &network_rx.borrow());
            }
        });

        let battery_icon_c = battery_icon.clone();
        let mut power_rx = ctx.power_rx.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            Self::update_battery(&battery_icon_c, &power_rx.borrow());
            while power_rx.changed().await.is_ok() {
                Self::update_battery(&battery_icon_c, &power_rx.borrow());
            }
        });

        Self {
            window,
            ws_label,
            clock_label,
            vol_icon,
            status_island: status_island.container,
            center_island: center_island.container,
        }
    }

    fn update_wifi(icon: &gtk4::Image, data: &crate::services::network::NetworkData) {
        let icon_name = if !data.is_wifi_enabled || !data.is_wifi_connected {
            "network-wireless-offline-symbolic"
        } else {
            if data.active_strength > 80 { "network-wireless-signal-excellent-symbolic" }
            else if data.active_strength > 60 { "network-wireless-signal-good-symbolic" }
            else if data.active_strength > 40 { "network-wireless-signal-ok-symbolic" }
            else { "network-wireless-signal-weak-symbolic" }
        };
        icon.set_icon_name(Some(icon_name));
    }

    fn update_battery(icon: &gtk4::Image, data: &crate::services::power::PowerData) {
        icon.set_visible(data.has_battery);
        if data.has_battery {
            let icon_name = if data.is_charging {
                "battery-full-charging-symbolic"
            } else {
                if data.battery_percentage < 10.0 { "battery-empty-symbolic" }
                else if data.battery_percentage < 30.0 { "battery-low-symbolic" }
                else if data.battery_percentage < 60.0 { "battery-good-symbolic" }
                else { "battery-full-symbolic" }
            };
            icon.set_icon_name(Some(icon_name));
        }
    }
}

use crate::app_context::AppContext;
use crate::widgets::Island;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

pub struct Bar {
    pub window: gtk4::ApplicationWindow,
    pub status_island: gtk4::Box,
    pub center_island: gtk4::Box,
    pub vol_icon: gtk4::Image,
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
        center_island
            .container
            .set_cursor_from_name(Some("pointer"));
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
        status_island
            .container
            .set_cursor_from_name(Some("pointer"));

        let wifi_icon = gtk4::Image::from_icon_name("network-wireless-symbolic");
        let bt_icon = gtk4::Image::from_icon_name("bluetooth-symbolic");
        let vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        let battery_icon = gtk4::Image::from_icon_name("battery-full-symbolic");

        status_island.append(&wifi_icon);
        status_island.append(&bt_icon);
        status_island.append(&vol_icon);
        status_island.append(&battery_icon);

        root.set_end_widget(Some(&status_island.container));
        window.set_child(Some(&root));

        // --- REAKTIVE BINDINGS ---

        ctx.clock.subscribe(move |time| {
            clock_label.set_text(&time.format("%H:%M").to_string());
        });

        ctx.niri.subscribe(move |data| {
            Self::update_workspaces(&ws_label, data);
        });

        ctx.network.subscribe(move |data| {
            Self::update_wifi(&wifi_icon, data);
        });

        ctx.bluetooth.subscribe(move |data| {
            Self::update_bluetooth(&bt_icon, data);
        });

        ctx.power.subscribe(move |data| {
            Self::update_battery(&battery_icon, data);
        });

        Self {
            window,
            status_island: status_island.container,
            center_island: center_island.container,
            vol_icon,
        }
    }

    fn update_workspaces(label: &gtk4::Label, data: &crate::services::niri::NiriData) {
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

    fn update_wifi(icon: &gtk4::Image, data: &crate::services::network::NetworkData) {
        icon.set_visible(data.is_wifi_enabled);
        if data.is_wifi_enabled {
            let icon_name = if !data.is_wifi_connected {
                "network-wireless-offline-symbolic"
            } else if data.active_strength > 80 {
                "network-wireless-signal-excellent-symbolic"
            } else if data.active_strength > 60 {
                "network-wireless-signal-good-symbolic"
            } else if data.active_strength > 40 {
                "network-wireless-signal-ok-symbolic"
            } else {
                "network-wireless-signal-weak-symbolic"
            };
            icon.set_icon_name(Some(icon_name));
        }
    }

    fn update_bluetooth(icon: &gtk4::Image, data: &crate::services::bluetooth::BluetoothData) {
        icon.set_visible(data.is_powered);
        if data.is_powered {
            let any_connected = data.devices.iter().any(|d| d.is_connected);
            let icon_name = if any_connected {
                "bluetooth-active-symbolic"
            } else {
                "bluetooth-symbolic"
            };
            icon.set_icon_name(Some(icon_name));
        }
    }

    fn update_battery(icon: &gtk4::Image, data: &crate::services::power::PowerData) {
        icon.set_visible(data.has_battery);
        if data.has_battery {
            let icon_name = if data.is_charging {
                "battery-full-charging-symbolic"
            } else if data.battery_percentage < 10.0 {
                "battery-empty-symbolic"
            } else if data.battery_percentage < 30.0 {
                "battery-low-symbolic"
            } else if data.battery_percentage < 60.0 {
                "battery-good-symbolic"
            } else {
                "battery-full-symbolic"
            };
            icon.set_icon_name(Some(icon_name));
        }
    }
}

use gtk4::prelude::*;
use crate::widgets::Island;
use crate::app_context::AppContext;

pub struct BarStatus {
    pub container: gtk4::Box,
    pub vol_icon: gtk4::Image,
}

impl BarStatus {
    pub fn new(ctx: AppContext) -> Self {
        let island = Island::new(12);
        island.container.set_cursor_from_name(Some("pointer"));

        let wifi_icon = gtk4::Image::from_icon_name("network-wireless-symbolic");
        let bt_icon = gtk4::Image::from_icon_name("bluetooth-symbolic");
        let vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        let battery_icon = gtk4::Image::from_icon_name("battery-full-symbolic");

        island.append(&wifi_icon);
        island.append(&bt_icon);
        island.append(&vol_icon);
        island.append(&battery_icon);

        // Subscriptions
        ctx.network.subscribe(move |data| {
            wifi_icon.set_visible(data.is_wifi_enabled);
            if data.is_wifi_enabled {
                let icon_name = if !data.is_wifi_connected { "network-wireless-offline-symbolic" }
                else if data.active_strength > 80 { "network-wireless-signal-excellent-symbolic" }
                else if data.active_strength > 60 { "network-wireless-signal-good-symbolic" }
                else if data.active_strength > 40 { "network-wireless-signal-ok-symbolic" }
                else { "network-wireless-signal-weak-symbolic" };
                wifi_icon.set_icon_name(Some(icon_name));
            }
        });

        ctx.bluetooth.subscribe(move |data| {
            bt_icon.set_visible(data.is_powered);
            if data.is_powered {
                let any_connected = data.devices.iter().any(|d| d.is_connected);
                let icon_name = if any_connected { "bluetooth-active-symbolic" } else { "bluetooth-symbolic" };
                bt_icon.set_icon_name(Some(icon_name));
            }
        });

        ctx.power.subscribe(move |data| {
            battery_icon.set_visible(data.has_battery);
            if data.has_battery {
                let icon_name = if data.is_charging { "battery-full-charging-symbolic" }
                else if data.battery_percentage < 10.0 { "battery-empty-symbolic" }
                else if data.battery_percentage < 30.0 { "battery-low-symbolic" }
                else if data.battery_percentage < 60.0 { "battery-good-symbolic" }
                else { "battery-full-symbolic" };
                battery_icon.set_icon_name(Some(icon_name));
            }
        });

        let vol_icon_clone = vol_icon.clone();
        ctx.audio.subscribe(move |data| {
            let icon_name = if data.is_muted || data.volume <= 0.01 { "audio-volume-muted-symbolic" }
            else if data.volume < 0.33 { "audio-volume-low-symbolic" }
            else if data.volume < 0.66 { "audio-volume-medium-symbolic" }
            else { "audio-volume-high-symbolic" };
            vol_icon_clone.set_icon_name(Some(icon_name));
        });

        Self {
            container: island.container,
            vol_icon,
        }
    }
}

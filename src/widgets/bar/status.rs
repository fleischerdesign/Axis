use crate::app_context::AppContext;
use crate::widgets::icons;
use crate::widgets::Island;
use gtk4::prelude::*;

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
        let notif_icon = gtk4::Image::from_icon_name("preferences-system-notifications-symbolic");
        notif_icon.set_visible(false);

        island.append(&wifi_icon);
        island.append(&bt_icon);
        island.append(&vol_icon);
        island.append(&battery_icon);
        island.append(&notif_icon);

        ctx.notifications.subscribe(move |data| {
            notif_icon.set_visible(!data.notifications.is_empty());
        });

        ctx.network.subscribe(move |data| {
            wifi_icon.set_visible(data.is_wifi_enabled);
            if data.is_wifi_enabled {
                let icon_name = if !data.is_wifi_connected {
                    "network-wireless-offline-symbolic"
                } else {
                    icons::wifi_signal_icon(data.active_strength)
                };
                wifi_icon.set_icon_name(Some(icon_name));
            }
        });

        ctx.bluetooth.subscribe(move |data| {
            bt_icon.set_visible(data.is_powered);
            if data.is_powered {
                let any_connected = data.devices.iter().any(|d| d.is_connected);
                let icon_name = if any_connected {
                    "bluetooth-active-symbolic"
                } else {
                    "bluetooth-symbolic"
                };
                bt_icon.set_icon_name(Some(icon_name));
            }
        });

        ctx.power.subscribe(move |data| {
            battery_icon.set_visible(data.has_battery);
            if data.has_battery {
                battery_icon.set_icon_name(Some(icons::battery_icon(
                    data.battery_percentage,
                    data.is_charging,
                )));
            }
        });

        let vol_icon_clone = vol_icon.clone();
        ctx.audio.subscribe(move |data| {
            vol_icon_clone.set_icon_name(Some(icons::volume_icon(data.volume, data.is_muted)));
        });

        Self {
            container: island.container,
            vol_icon,
        }
    }
}

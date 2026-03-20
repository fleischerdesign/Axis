use crate::app_context::AppContext;
use crate::widgets::icons;
use crate::widgets::icons::battery::BatteryIcon;
use crate::widgets::icons::bt::BtIcon;
use crate::widgets::icons::wifi::WifiIcon;
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

        let wifi = WifiIcon::new(&ctx);
        let bt = BtIcon::new(&ctx);
        let battery = BatteryIcon::new(&ctx);

        let vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        let notif_icon = gtk4::Image::from_icon_name("preferences-system-notifications-symbolic");
        notif_icon.set_visible(false);

        island.append(&wifi.icon.image);
        island.append(&bt.icon.image);
        island.append(&vol_icon);
        island.append(&battery.icon.image);
        island.append(&notif_icon);

        ctx.notifications.subscribe(move |data| {
            notif_icon.set_visible(!data.notifications.is_empty());
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

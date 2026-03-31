use super::base::BaseIcon;
use crate::app_context::AppContext;
use crate::widgets::icons;

#[derive(Clone)]
pub struct WifiIcon {
    pub icon: BaseIcon,
}

impl WifiIcon {
    pub fn new(ctx: &AppContext) -> Self {
        let icon = BaseIcon::new("network-wireless-symbolic");
        let icon_c = icon.clone();
        Self::on_change(ctx, move |name, visible| {
            icon_c.set_visible(visible);
            if visible {
                icon_c.set(name);
            }
        });
        Self { icon }
    }

    pub fn on_change(ctx: &AppContext, f: impl Fn(&str, bool) + 'static) {
        ctx.network.subscribe(move |data| {
            if !data.is_wifi_enabled {
                f("network-wireless-disabled-symbolic", true);
            } else if !data.is_wifi_connected {
                f("network-wireless-offline-symbolic", true);
            } else {
                f(icons::wifi_signal_icon(data.active_strength), true);
            }
        });
    }
}

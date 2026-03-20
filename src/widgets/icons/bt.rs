use super::base::BaseIcon;
use crate::app_context::AppContext;

#[derive(Clone)]
pub struct BtIcon {
    pub icon: BaseIcon,
}

impl BtIcon {
    pub fn new(ctx: &AppContext) -> Self {
        let icon = BaseIcon::new("bluetooth-symbolic");
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
        ctx.bluetooth.subscribe(move |data| {
            if !data.is_powered {
                f("bluetooth-disabled-symbolic", true);
            } else {
                let any_connected = data.devices.iter().any(|d| d.is_connected);
                let name = if any_connected {
                    "bluetooth-active-symbolic"
                } else {
                    "bluetooth-symbolic"
                };
                f(name, true);
            }
        });
    }
}

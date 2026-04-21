use super::base::BaseIcon;
use crate::app_context::AppContext;
use crate::widgets::icons;

#[derive(Clone)]
pub struct BatteryIcon {
    pub icon: BaseIcon,
}

impl BatteryIcon {
    pub fn new(ctx: &AppContext) -> Self {
        let icon = BaseIcon::new("battery-full-symbolic");
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
        ctx.power.subscribe(move |data| {
            if !data.has_battery {
                f("battery-full-symbolic", false);
            } else {
                f(
                    icons::battery_icon(data.battery_percentage, data.is_charging),
                    true,
                );
            }
        });
    }
}

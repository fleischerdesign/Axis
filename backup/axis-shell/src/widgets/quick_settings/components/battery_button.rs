use crate::app_context::AppContext;
use crate::widgets::icons;
use gtk4::prelude::*;

pub struct BatteryButton {
    pub btn: gtk4::Button,
}

impl BatteryButton {
    pub fn new(ctx: &AppContext) -> Self {
        let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let icon = gtk4::Image::from_icon_name("battery-full-symbolic");
        let label = gtk4::Label::new(Some("...%"));
        content.append(&icon);
        content.append(&label);

        let btn = gtk4::Button::builder()
            .child(&content)
            .css_classes(vec!["qs-battery-btn".to_string()])
            .build();

        let btn_c = btn.clone();
        let icon_c = icon.clone();
        let label_c = label.clone();
        ctx.power.subscribe(move |data| {
            btn_c.set_visible(data.has_battery);
            if data.has_battery {
                label_c.set_text(&format!("{:.0}%", data.battery_percentage));
                icon_c.set_icon_name(Some(icons::battery_icon(
                    data.battery_percentage,
                    data.is_charging,
                )));
            }
        });

        Self { btn }
    }
}

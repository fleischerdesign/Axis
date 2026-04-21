use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use axis_domain::models::power::PowerStatus;
use crate::presentation::battery::{BatteryView, battery_icon};
use axis_presentation::View;

glib::wrapper! {
    pub struct BatteryButton(ObjectSubclass<imp::BatteryButton>)
        @extends gtk4::Widget, gtk4::Button,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl BatteryButton {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl View<PowerStatus> for BatteryButton {
    fn render(&self, status: &PowerStatus) {
        let visible = status.has_battery;
        let icon_name = battery_icon(status.battery_percentage, status.is_charging).to_string();
        let pct_text = format!("{:.0}%", status.battery_percentage);
        let btn = self.clone();

        glib::idle_add_local(move || {
            btn.set_visible(visible);
            if visible {
                btn.imp().icon.set_icon_name(Some(&icon_name));
                btn.imp().label.set_label(&pct_text);
            }
            glib::ControlFlow::Break
        });
    }
}

impl BatteryView for BatteryButton {}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct BatteryButton {
        pub icon: gtk4::Image,
        pub label: gtk4::Label,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BatteryButton {
        const NAME: &'static str = "AxisBatteryButton";
        type Type = super::BatteryButton;
        type ParentType = gtk4::Button;
    }

    impl ObjectImpl for BatteryButton {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            self.icon.set_pixel_size(20);
            content.append(&self.icon);
            content.append(&self.label);

            obj.set_child(Some(&content));
            obj.add_css_class("qs-battery-btn");
            obj.set_visible(false);
        }
    }

    impl WidgetImpl for BatteryButton {}
    impl ButtonImpl for BatteryButton {}
}

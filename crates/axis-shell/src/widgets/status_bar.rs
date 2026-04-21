use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use axis_presentation::View;
use crate::presentation::battery::{BatteryView, battery_icon};
use axis_domain::models::power::PowerStatus;

glib::wrapper! {
    pub struct StatusBar(ObjectSubclass<imp::StatusBar>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl StatusBar {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl View<PowerStatus> for StatusBar {
    fn render(&self, status: &PowerStatus) {
        let icon_name = battery_icon(status.battery_percentage, status.is_charging).to_string();
        let percentage_text = format!("{:.0}%", status.battery_percentage);
        let is_charging = status.is_charging;
        let icon = self.imp().icon.clone();
        let label = self.imp().label.clone();

        glib::idle_add_local(move || {
            icon.set_icon_name(Some(&icon_name));
            label.set_label(&percentage_text);

            if is_charging {
                icon.add_css_class("charging");
            } else {
                icon.remove_css_class("charging");
            }
            glib::ControlFlow::Break
        });
    }
}

impl BatteryView for StatusBar {}

pub mod imp {
    use super::*;

    #[derive(Default)]
    pub struct StatusBar {
        pub icon: gtk4::Image,
        pub label: gtk4::Label,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatusBar {
        const NAME: &'static str = "StatusBar";
        type Type = super::StatusBar;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for StatusBar {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_spacing(4);
            self.icon.set_pixel_size(20);
            self.icon.add_css_class("status-icon");
            self.label.add_css_class("status-text");
            self.label.set_visible(false);
            self.obj().append(&self.icon);
            self.obj().append(&self.label);
            self.obj().add_css_class("status-bar");
        }
    }

    impl WidgetImpl for StatusBar {}
    impl BoxImpl for StatusBar {}
}

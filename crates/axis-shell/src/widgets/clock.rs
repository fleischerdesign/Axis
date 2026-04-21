use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use axis_presentation::View;
use axis_domain::models::clock::TimeStatus;

glib::wrapper! {
    pub struct ClockWidget(ObjectSubclass<imp::ClockWidget>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl ClockWidget {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl View<TimeStatus> for ClockWidget {
    fn render(&self, status: &TimeStatus) {
        let label = self.imp().label.clone();
        let time_str = status.current_time.format("%H:%M:%S").to_string();
        glib::idle_add_local(move || {
            label.set_label(&time_str);
            glib::ControlFlow::Break
        });
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ClockWidget {
        pub label: gtk4::Label,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ClockWidget {
        const NAME: &'static str = "ClockWidget";
        type Type = super::ClockWidget;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for ClockWidget {
        fn constructed(&self) {
            self.parent_constructed();
            self.label.add_css_class("clock-label");
            self.obj().append(&self.label);
        }
    }

    impl WidgetImpl for ClockWidget {}
    impl BoxImpl for ClockWidget {}
}

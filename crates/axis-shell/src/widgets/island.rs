use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;

glib::wrapper! {
    pub struct Island(ObjectSubclass<imp::Island>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Island {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn on_clicked<F: Fn() + 'static>(&self, f: F) {
        let gesture = gtk4::GestureClick::new();
        gesture.connect_released(move |_, _, _, _| {
            f();
        });
        self.add_controller(gesture);
        self.set_cursor_from_name(Some("pointer"));
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Island;

    #[glib::object_subclass]
    impl ObjectSubclass for Island {
        const NAME: &'static str = "Island";
        type Type = super::Island;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for Island {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_spacing(8);
            self.obj().add_css_class("island");
        }
    }

    impl WidgetImpl for Island {}
    impl BoxImpl for Island {}
}

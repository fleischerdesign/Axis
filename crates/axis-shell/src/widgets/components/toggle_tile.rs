use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;

glib::wrapper! {
    pub struct ToggleTile(ObjectSubclass<imp::ToggleTile>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl ToggleTile {
    pub fn new(label: &str, icon_name: &str, has_arrow: bool) -> Self {
        let obj: Self = glib::Object::new();
        obj.set_label(label);
        obj.set_icon(icon_name);
        obj.set_show_arrow(has_arrow);
        obj
    }

    pub fn set_label(&self, label: &str) {
        self.imp().label.set_label(label);
    }

    pub fn set_icon(&self, icon_name: &str) {
        self.imp().icon.set_icon_name(Some(icon_name));
    }

    pub fn set_active(&self, active: bool) {
        if active {
            self.add_css_class("active");
        } else {
            self.remove_css_class("active");
        }
    }

    pub fn set_show_arrow(&self, show: bool) {
        self.imp().arrow_btn.set_visible(show);
        if show {
            self.imp().main_btn.remove_css_class("sole");
        } else {
            self.imp().main_btn.add_css_class("sole");
        }
    }

    pub fn on_clicked<F: Fn() + 'static>(&self, f: F) {
        self.imp().main_btn.connect_clicked(move |_| {
            f();
        });
    }

    pub fn on_arrow_clicked<F: Fn() + 'static>(&self, f: F) {
        self.imp().arrow_btn.connect_clicked(move |_| {
            f();
        });
    }
}

impl crate::presentation::toggle::ToggleView for ToggleTile {
    fn set_active(&self, active: bool) {
        self.set_active(active);
    }

    fn set_icon(&self, icon_name: &str) {
        self.set_icon(icon_name);
    }

    fn set_label(&self, label: &str) {
        self.set_label(label);
    }

    fn on_toggled(&self, f: Box<dyn Fn(bool) + 'static>) {
        let tile = self.clone();
        self.on_clicked(move || {
            let next_state = !tile.has_css_class("active");
            f(next_state);
        });
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ToggleTile {
        pub main_btn: gtk4::Button,
        pub arrow_btn: gtk4::Button,
        pub icon: gtk4::Image,
        pub label: gtk4::Label,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ToggleTile {
        const NAME: &'static str = "ToggleTile";
        type Type = super::ToggleTile;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for ToggleTile {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.set_spacing(0);
            obj.add_css_class("tile");

            self.main_btn.add_css_class("tile-main");
            self.main_btn.set_hexpand(true);
            self.main_btn.add_css_class("sole"); // Default

            let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            self.icon.set_pixel_size(18);
            self.label.add_css_class("tile-label");
            
            content.append(&self.icon);
            content.append(&self.label);
            self.main_btn.set_child(Some(&content));

            self.arrow_btn.set_icon_name("go-next-symbolic");
            self.arrow_btn.add_css_class("tile-arrow");
            self.arrow_btn.set_visible(false);

            obj.append(&self.main_btn);
            obj.append(&self.arrow_btn);
        }
    }

    impl WidgetImpl for ToggleTile {}
    impl BoxImpl for ToggleTile {}
}

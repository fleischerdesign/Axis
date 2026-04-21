use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;

glib::wrapper! {
    pub struct LauncherWidget(ObjectSubclass<imp::LauncherWidget>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl LauncherWidget {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct LauncherWidget;

    #[glib::object_subclass]
    impl ObjectSubclass for LauncherWidget {
        const NAME: &'static str = "LauncherWidget";
        type Type = super::LauncherWidget;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for LauncherWidget {
        fn constructed(&self) {
            self.parent_constructed();
            
            let icon = gtk4::Image::from_icon_name("view-app-grid-symbolic");
            icon.set_pixel_size(20);
            icon.add_css_class("status-icon");
            
            self.obj().append(&icon);
            self.obj().add_css_class("launcher-widget");
        }
    }

    impl WidgetImpl for LauncherWidget {}
    impl BoxImpl for LauncherWidget {}
}

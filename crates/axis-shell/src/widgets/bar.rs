use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;

glib::wrapper! {
    pub struct Bar(ObjectSubclass<imp::Bar>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Bar {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_start_widget<P: IsA<gtk4::Widget>>(&self, widget: Option<&P>) {
        self.imp().center_box.set_start_widget(widget);
    }

    pub fn set_center_widget<P: IsA<gtk4::Widget>>(&self, widget: Option<&P>) {
        self.imp().center_box.set_center_widget(widget);
    }

    pub fn set_end_widget<P: IsA<gtk4::Widget>>(&self, widget: Option<&P>) {
        self.imp().center_box.set_end_widget(widget);
    }
}

mod imp {
    use super::*;

    pub struct Bar {
        pub center_box: gtk4::CenterBox,
    }

    impl Default for Bar {
        fn default() -> Self {
            Self {
                center_box: gtk4::CenterBox::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Bar {
        const NAME: &'static str = "Bar";
        type Type = super::Bar;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for Bar {
        fn constructed(&self) {
            self.parent_constructed();
            
            // Die CenterBox enthält die Inseln
            self.center_box.set_hexpand(true);
            self.center_box.set_valign(gtk4::Align::Start); // Inseln kleben oben im Fenster
            self.center_box.set_height_request(44);
            self.center_box.add_css_class("bar-container");
            
            // Das Bar-Widget selbst füllt die vollen 54px aus (inkl. 10px Lücke unten)
            self.obj().set_height_request(54);
            self.obj().set_vexpand(true);
            
            self.obj().append(&self.center_box);
            self.obj().add_css_class("bar-main-widget");
        }
    }

    impl WidgetImpl for Bar {}
    impl BoxImpl for Bar {}
}

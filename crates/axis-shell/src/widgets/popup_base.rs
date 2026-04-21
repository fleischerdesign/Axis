use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;

glib::wrapper! {
    pub struct PopupContainer(ObjectSubclass<imp::PopupContainer>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl PopupContainer {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_content(&self, widget: &impl IsA<gtk4::Widget>) {
        self.imp().revealer.set_child(Some(widget));
    }

    pub fn prepend_outside(&self, widget: &impl IsA<gtk4::Widget>) {
        self.prepend(widget);
    }

    pub fn set_reveal(&self, reveal: bool) {
        self.imp().revealer.set_reveal_child(reveal);
    }

    pub fn animate_show(&self, window: &gtk4::ApplicationWindow) {
        let wrapper = self.imp().wrapper.clone();
        let container = self.clone();
        let window = window.clone();

        glib::idle_add_local(move || {
            wrapper.add_css_class("popup-hiding");
            window.set_visible(true);
            container.imp().revealer.set_reveal_child(true);

            let w = wrapper.clone();
            glib::idle_add_local(move || {
                w.remove_css_class("popup-hiding");
                glib::ControlFlow::Break
            });
            glib::ControlFlow::Break
        });
    }

    pub fn animate_hide(&self, window: &gtk4::ApplicationWindow) {
        let wrapper = self.imp().wrapper.clone();
        let container = self.clone();
        let window = window.clone();

        glib::idle_add_local(move || {
            wrapper.add_css_class("popup-hiding");
            container.imp().revealer.set_reveal_child(false);

            let w = wrapper.clone();
            let win = window.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(260), move || {
                win.set_visible(false);
                w.remove_css_class("popup-hiding");
            });
            glib::ControlFlow::Break
        });
    }
}

mod imp {
    use super::*;

    pub struct PopupContainer {
        pub wrapper: gtk4::Box,
        pub revealer: gtk4::Revealer,
    }

    impl Default for PopupContainer {
        fn default() -> Self {
            Self {
                wrapper: gtk4::Box::new(gtk4::Orientation::Vertical, 0),
                revealer: gtk4::Revealer::builder()
                    .transition_type(gtk4::RevealerTransitionType::SlideUp)
                    .transition_duration(250)
                    .build(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PopupContainer {
        const NAME: &'static str = "PopupContainer";
        type Type = super::PopupContainer;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for PopupContainer {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_orientation(gtk4::Orientation::Vertical);
            obj.set_valign(gtk4::Align::End);

            self.wrapper.add_css_class("popup-content");
            self.wrapper.append(&self.revealer);

            obj.append(&self.wrapper);
        }
    }

    impl WidgetImpl for PopupContainer {}
    impl BoxImpl for PopupContainer {}
}

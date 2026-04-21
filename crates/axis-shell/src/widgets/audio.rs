use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use crate::presentation::presenter::View;
use crate::presentation::audio::{AudioView, audio_icon};
use axis_domain::models::audio::AudioStatus;

glib::wrapper! {
    pub struct AudioWidget(ObjectSubclass<imp::AudioWidget>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl AudioWidget {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl View<AudioStatus> for AudioWidget {
    fn render(&self, status: &AudioStatus) {
        let icon_name = audio_icon(status).to_string();
        let icon = self.imp().icon.clone();

        glib::idle_add_local(move || {
            icon.set_icon_name(Some(&icon_name));
            glib::ControlFlow::Break
        });
    }
}

impl AudioView for AudioWidget {
    fn on_volume_changed(&self, _f: Box<dyn Fn(f64) + 'static>) {}
    fn on_set_default_sink(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_default_source(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_sink_input_volume(&self, _f: Box<dyn Fn(u32, f64) + 'static>) {}
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct AudioWidget {
        pub icon: gtk4::Image,
        pub label: gtk4::Label,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AudioWidget {
        const NAME: &'static str = "AudioWidget";
        type Type = super::AudioWidget;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for AudioWidget {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_spacing(4);
            self.icon.set_pixel_size(20);
            self.icon.add_css_class("status-icon");
            self.label.add_css_class("status-text");
            self.label.set_visible(false);
            self.obj().append(&self.icon);
            self.obj().append(&self.label);
            self.obj().add_css_class("audio-widget");
        }
    }

    impl WidgetImpl for AudioWidget {}
    impl BoxImpl for AudioWidget {}
}

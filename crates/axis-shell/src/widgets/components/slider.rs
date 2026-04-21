use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;

glib::wrapper! {
    pub struct QuickSlider(ObjectSubclass<imp::QuickSlider>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl QuickSlider {
    pub fn new(icon_name: &str) -> Self {
        let obj: Self = glib::Object::new();
        obj.set_icon(icon_name);
        obj
    }

    pub fn set_icon(&self, icon_name: &str) {
        self.imp().icon.set_icon_name(Some(icon_name));
    }

    pub fn scale(&self) -> gtk4::Scale {
        self.imp().scale.clone()
    }

    pub fn set_show_arrow(&self, show: bool) {
        self.imp().arrow_btn.set_visible(show);
        if show {
            self.imp().scale.add_css_class("with-arrow");
        } else {
            self.imp().scale.remove_css_class("with-arrow");
        }
    }

    pub fn on_arrow_clicked<F: Fn() + 'static>(&self, f: F) {
        self.imp().arrow_btn.connect_clicked(move |_| {
            f();
        });
    }
}

mod imp {
    use super::*;

    pub struct QuickSlider {
        pub overlay: gtk4::Overlay,
        pub icon: gtk4::Image,
        pub scale: gtk4::Scale,
        pub arrow_btn: gtk4::Button,
    }

    impl Default for QuickSlider {
        fn default() -> Self {
            Self {
                overlay: gtk4::Overlay::new(),
                icon: gtk4::Image::new(),
                scale: gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01),
                arrow_btn: gtk4::Button::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for QuickSlider {
        const NAME: &'static str = "QuickSlider";
        type Type = super::QuickSlider;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for QuickSlider {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.set_spacing(0);
            obj.add_css_class("slider-row");
            obj.add_css_class("volume-slider");

            // Icon Setup (über dem Slider schwebend)
            self.icon.set_pixel_size(22);
            self.icon.set_halign(gtk4::Align::Start);
            self.icon.set_valign(gtk4::Align::Center);
            self.icon.set_margin_start(22);
            self.icon.set_can_target(false); // Klicks gehen durch das Icon zum Slider
            self.icon.add_css_class("slider-icon-overlay");

            // Slider Setup
            self.scale.set_draw_value(false);
            self.scale.set_hexpand(true);
            self.scale.set_valign(gtk4::Align::Center);

            // Arrow Setup
            self.arrow_btn.set_icon_name("go-next-symbolic");
            self.arrow_btn.add_css_class("tile-arrow");
            self.arrow_btn.set_visible(false);

            // Overlay zusammenbauen
            self.overlay.set_child(Some(&self.scale));
            self.overlay.add_overlay(&self.icon);
            self.overlay.set_hexpand(true);

            // Logik für das "Einfärben" des Buttons und Abrunden des Sliders bei 100%
            let arrow_c = self.arrow_btn.clone();
            let scale_c = self.scale.clone();
            self.scale.connect_value_changed(move |s| {
                let is_full = s.value() >= s.adjustment().upper() - 0.01;
                
                if is_full {
                    arrow_c.add_css_class("max");
                    scale_c.remove_css_class("highlight-partial");
                } else {
                    arrow_c.remove_css_class("max");
                    scale_c.add_css_class("highlight-partial");
                }
            });

            // Initialen Zustand setzen
            self.scale.add_css_class("highlight-partial");

            obj.append(&self.overlay);
            obj.append(&self.arrow_btn);
        }
    }

    impl WidgetImpl for QuickSlider {}
    impl BoxImpl for QuickSlider {}
}

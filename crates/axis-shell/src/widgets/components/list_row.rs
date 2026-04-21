use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;

glib::wrapper! {
    pub struct ListRow(ObjectSubclass<imp::ListRow>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl ListRow {
    pub fn new(title: &str, icon: &str) -> Self {
        let obj: Self = glib::Object::new();
        obj.set_title(title);
        obj.set_icon(icon);
        obj
    }

    pub fn set_title(&self, title: &str) {
        self.imp().title.set_label(title);
    }

    pub fn set_icon(&self, icon: &str) {
        self.imp().icon.set_icon_name(Some(icon));
    }

    pub fn set_subtitle(&self, text: Option<&str>) {
        let imp = self.imp();
        if let Some(t) = text {
            imp.subtitle.set_label(t);
            imp.subtitle.set_visible(true);
        } else {
            imp.subtitle.set_visible(false);
        }
    }

    pub fn set_active(&self, active: bool) {
        if active {
            self.add_css_class("active");
        } else {
            self.remove_css_class("active");
        }
    }

    pub fn set_show_checkmark(&self, show: bool) {
        self.imp().checkmark.set_visible(show);
    }

    pub fn set_trailing(&self, widget: Option<&gtk4::Widget>) {
        let imp = self.imp();
        if let Some(w) = widget {
            imp.trailing.append(w);
            imp.trailing.set_visible(true);
        } else {
            while let Some(child) = imp.trailing.first_child() {
                imp.trailing.remove(&child);
            }
            imp.trailing.set_visible(false);
        }
    }
}

mod imp {
    use super::*;

    pub struct ListRow {
        pub icon: gtk4::Image,
        pub title: gtk4::Label,
        pub subtitle: gtk4::Label,
        pub checkmark: gtk4::Image,
        pub trailing: gtk4::Box,
    }

    impl Default for ListRow {
        fn default() -> Self {
            let icon = gtk4::Image::new();
            icon.set_pixel_size(18);

            let title = gtk4::Label::builder()
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();

            let subtitle = gtk4::Label::builder()
                .halign(gtk4::Align::Start)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .max_width_chars(45)
                .css_classes(vec!["list-sublabel"])
                .visible(false)
                .build();

            let checkmark = gtk4::Image::from_icon_name("object-select-symbolic");
            checkmark.set_halign(gtk4::Align::End);
            checkmark.set_valign(gtk4::Align::Center);
            checkmark.set_visible(false);

            let trailing = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
            trailing.set_visible(false);

            Self { icon, title, subtitle, checkmark, trailing }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ListRow {
        const NAME: &'static str = "AxisListRow";
        type Type = super::ListRow;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for ListRow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.set_margin_start(12);
            obj.set_margin_end(12);
            obj.set_margin_top(8);
            obj.set_margin_bottom(8);
            obj.set_spacing(12);
            obj.add_css_class("list-row");

            let label_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            label_box.set_hexpand(true);
            label_box.append(&self.title);
            label_box.append(&self.subtitle);

            obj.append(&self.icon);
            obj.append(&label_box);
            obj.append(&self.trailing);
            obj.append(&self.checkmark);
        }
    }

    impl WidgetImpl for ListRow {}
    impl BoxImpl for ListRow {}
}

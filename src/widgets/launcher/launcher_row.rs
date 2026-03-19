use crate::services::launcher::provider::LauncherItem;
use gtk4::prelude::*;

/// A `gtk4::ListBoxRow`-based widget for a single launcher result.
/// Using `ListBoxRow` as the root allows `ListBox::select_row()` and
/// GTK's built-in keyboard-scroll to work correctly without any manual
/// adjustment hacks.
pub struct LauncherRow {
    pub row: gtk4::ListBoxRow,
}

impl LauncherRow {
    pub fn new(item: &LauncherItem) -> Self {
        let row = gtk4::ListBoxRow::builder()
            .css_classes(vec!["launcher-row"])
            .build();

        let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(8);
        content.set_margin_bottom(8);

        let icon = gtk4::Image::from_icon_name(&item.icon_name);
        icon.set_pixel_size(18);
        content.append(&icon);

        let label_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        label_box.set_hexpand(true);

        let title = gtk4::Label::builder()
            .label(&item.title)
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        label_box.append(&title);

        if let Some(desc) = &item.description {
            let subtitle = gtk4::Label::builder()
                .label(desc)
                .halign(gtk4::Align::Start)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .max_width_chars(45)
                .css_classes(vec!["list-sublabel"])
                .build();
            label_box.append(&subtitle);
        }

        content.append(&label_box);
        row.set_child(Some(&content));
        row.set_cursor_from_name(Some("pointer"));

        Self { row }
    }
}

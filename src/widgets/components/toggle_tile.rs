use crate::store::ServiceHandle;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct ToggleTile {
    pub container: gtk4::Box,
    pub main_btn: gtk4::Button,
    pub arrow_btn: Option<gtk4::Button>,
    pub icon_img: gtk4::Image,
}

impl ToggleTile {
    pub fn new(label: &str, icon: &str, has_arrow: bool) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.add_css_class("tile");

        let main_btn = gtk4::Button::builder()
            .css_classes(vec!["tile-main".to_string()])
            .hexpand(true)
            .build();
        if !has_arrow {
            main_btn.add_css_class("sole");
        }

        let main_content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        let icon_img = gtk4::Image::from_icon_name(icon);
        icon_img.set_pixel_size(18);
        let text_label = gtk4::Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .css_classes(vec!["tile-label".to_string()])
            .build();

        main_content.append(&icon_img);
        main_content.append(&text_label);
        main_btn.set_child(Some(&main_content));
        container.append(&main_btn);

        let mut arrow_btn = None;
        if has_arrow {
            let arrow = gtk4::Button::builder()
                .icon_name("go-next-symbolic")
                .css_classes(vec!["tile-arrow".to_string()])
                .build();
            container.append(&arrow);
            arrow_btn = Some(arrow);
        }

        Self {
            container,
            main_btn,
            arrow_btn,
            icon_img,
        }
    }

    pub fn set_active(&self, active: bool) {
        if active {
            self.container.add_css_class("active");
        } else {
            self.container.remove_css_class("active");
        }
    }

    pub fn set_icon(&self, icon: &str) {
        self.icon_img.set_icon_name(Some(icon));
    }

    pub fn set_sensitive(&self, sensitive: bool) {
        self.main_btn.set_sensitive(sensitive);
        if let Some(arrow) = &self.arrow_btn {
            arrow.set_sensitive(sensitive);
        }
    }

    /// Wires toggle handler, arrow handler, and store subscription in one call.
    ///
    /// - `handle`: the service's `ServiceHandle`
    /// - `make_cmd`: builds a toggle command from the desired state
    /// - `is_active`: extracts the boolean active state from service data
    /// - `on_arrow`: called when the arrow button is clicked
    /// - `on_subscribe`: extra logic called on each store update (besides set_active)
    pub fn wire_service<D, C, FCmd, FA, FArr, FS>(
        tile: &Rc<Self>,
        handle: &ServiceHandle<D, C>,
        make_cmd: FCmd,
        is_active: FA,
        on_arrow: FArr,
        on_subscribe: FS,
    ) where
        D: Clone + PartialEq + 'static,
        C: Send + 'static,
        FCmd: Fn(bool) -> C + 'static,
        FA: Fn(&D) -> bool + 'static,
        FArr: Fn() + 'static,
        FS: Fn(&ToggleTile, &D) + 'static,
    {
        let is_active = Rc::new(is_active);

        // Toggle handler
        let tx = handle.tx.clone();
        let store = handle.store.clone();
        let is_active_toggle = is_active.clone();
        tile.main_btn.connect_clicked(move |_| {
            let current = is_active_toggle(&store.get());
            let _ = tx.try_send(make_cmd(!current));
        });

        // Arrow handler
        if let Some(arrow) = &tile.arrow_btn {
            arrow.connect_clicked(move |_| {
                on_arrow();
            });
        }

        // Subscribe handler
        let tile_c = tile.clone();
        handle.subscribe(move |data| {
            tile_c.set_active(is_active(data));
            on_subscribe(&tile_c, data);
        });
    }
}

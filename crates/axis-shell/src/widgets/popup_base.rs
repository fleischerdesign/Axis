use libadwaita::prelude::*;
use gtk4::glib;

#[derive(Clone)]
pub struct PopupContainer {
    pub container: gtk4::Box,
    wrapper: gtk4::Box,
    revealer: gtk4::Revealer,
}

impl PopupContainer {
    pub fn new() -> Self {
        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .transition_duration(250)
            .build();

        let wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        wrapper.add_css_class("popup-content");
        wrapper.append(&revealer);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.set_valign(gtk4::Align::End);
        container.append(&wrapper);

        Self { container, wrapper, revealer }
    }

    pub fn set_content(&self, widget: &impl IsA<gtk4::Widget>) {
        self.revealer.set_child(Some(widget));
    }

    pub fn prepend_outside(&self, widget: &impl IsA<gtk4::Widget>) {
        self.container.prepend(widget);
    }

    pub fn set_width_request(&self, width: i32) {
        self.container.set_width_request(width);
    }

    pub fn animate_show(&self, window: &gtk4::ApplicationWindow) {
        let wrapper = self.wrapper.clone();
        let revealer = self.revealer.clone();
        let window = window.clone();

        glib::idle_add_local(move || {
            wrapper.add_css_class("popup-hiding");
            window.set_visible(true);
            revealer.set_reveal_child(true);

            let w = wrapper.clone();
            glib::idle_add_local(move || {
                w.remove_css_class("popup-hiding");
                glib::ControlFlow::Break
            });
            glib::ControlFlow::Break
        });
    }

    pub fn animate_hide(&self, window: &gtk4::ApplicationWindow) {
        let wrapper = self.wrapper.clone();
        let revealer = self.revealer.clone();
        let window = window.clone();

        glib::idle_add_local(move || {
            wrapper.add_css_class("popup-hiding");
            revealer.set_reveal_child(false);

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

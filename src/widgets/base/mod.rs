use crate::constants::REVEALER_TRANSITION_MS;
use crate::store::ReactiveBool;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::time::Duration;

#[derive(Clone)]
pub struct PopupBase {
    pub window: gtk4::Window,
    pub revealer: gtk4::Revealer,
    pub is_open: ReactiveBool,
}

impl PopupBase {
    /// General constructor. `anchor_right` for right-anchored popups.
    pub fn new(app: &libadwaita::Application, title: &str, anchor_right: bool) -> Self {
        let is_open = ReactiveBool::new(false);

        let window = gtk4::Window::builder()
            .application(app)
            .title(title)
            .visible(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_anchor(Edge::Bottom, true);
        window.set_default_size(380, -1);

        if anchor_right {
            window.set_anchor(Edge::Right, true);
            window.set_margin(Edge::Right, 10);
        } else {
            window.set_anchor(Edge::Left, true);
            window.set_margin(Edge::Left, 10);
        }

        window.set_margin(Edge::Bottom, 64);

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();

        window.set_child(Some(&revealer));

        Self {
            window,
            revealer,
            is_open,
        }
    }

    /// Centered popup (no left/right anchor).
    pub fn new_centered(app: &libadwaita::Application, title: &str) -> Self {
        let is_open = ReactiveBool::new(false);

        let window = gtk4::Window::builder()
            .application(app)
            .title(title)
            .visible(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Right, false);
        window.set_default_size(380, -1);
        window.set_margin(Edge::Bottom, 64);

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();

        window.set_child(Some(&revealer));

        Self {
            window,
            revealer,
            is_open,
        }
    }

    pub fn set_content(&self, content: &impl IsA<gtk4::Widget>) {
        self.revealer.set_child(Some(content));
    }

    pub fn content(&self) -> Option<gtk4::Widget> {
        self.revealer.child()
    }

    pub fn open(&self) {
        if self.is_open.get() {
            return;
        }
        self.is_open.set(true);
        self.window.set_visible(true);
        self.revealer.set_reveal_child(true);

        let window = self.window.clone();
        gtk4::glib::timeout_add_local_once(Duration::from_millis(50), move || {
            window.grab_focus();
        });
    }

    pub fn close(&self) {
        if !self.is_open.get() {
            return;
        }
        self.is_open.set(false);
        self.revealer.set_reveal_child(false);

        let win = self.window.clone();
        gtk4::glib::timeout_add_local(
            Duration::from_millis(REVEALER_TRANSITION_MS as u64),
            move || {
                win.set_visible(false);
                gtk4::glib::ControlFlow::Break
            },
        );
    }
}

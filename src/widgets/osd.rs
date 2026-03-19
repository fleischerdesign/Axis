use crate::app_context::AppContext;
use crate::widgets::icons;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

struct OsdModule {
    container: gtk4::Revealer,
    level_bar: gtk4::LevelBar,
    icon: gtk4::Image,
}

impl OsdModule {
    fn new(icon_name: &str) -> Self {
        let container = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .reveal_child(false)
            .build();

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        content.add_css_class("osd-module");
        content.set_valign(gtk4::Align::Center);
        content.set_halign(gtk4::Align::Center);
        content.set_height_request(160);
        content.set_width_request(44);

        let level_bar = gtk4::LevelBar::builder()
            .orientation(gtk4::Orientation::Vertical)
            .inverted(true)
            .min_value(0.0)
            .max_value(1.0)
            .valign(gtk4::Align::Fill)
            .halign(gtk4::Align::Center)
            .vexpand(true)
            .build();
        level_bar.add_css_class("osd-level-bar");

        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(16);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::End);

        content.append(&level_bar);
        content.append(&icon);
        container.set_child(Some(&content));

        Self {
            container,
            level_bar,
            icon,
        }
    }

    fn show(&self, value: f64, icon_name: &str) {
        self.level_bar.set_value(value);
        self.icon.set_icon_name(Some(icon_name));
        self.container.set_reveal_child(true);
    }

    fn hide(&self) {
        self.container.set_reveal_child(false);
    }

    fn is_active(&self) -> bool {
        self.container.reveals_child()
    }
}

pub struct OsdManager {
    window: gtk4::ApplicationWindow,
    vol_module: Rc<OsdModule>,
    bright_module: Rc<OsdModule>,
    hide_timeout: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
}

impl OsdManager {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Rc<Self> {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("OSD")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Right, 10);
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
        window.set_can_focus(false);

        let main_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        let vol_module = Rc::new(OsdModule::new("audio-volume-high-symbolic"));
        let bright_module = Rc::new(OsdModule::new("display-brightness-high-symbolic"));

        main_box.append(&bright_module.container);
        main_box.append(&vol_module.container);

        window.set_child(Some(&main_box));

        let manager = Rc::new(Self {
            window,
            vol_module,
            bright_module,
            hide_timeout: Rc::new(RefCell::new(None)),
        });

        manager.setup_subscriptions(ctx);

        manager
    }

    fn reset_hide_timeout(&self) {
        // Bestehenden Timeout canceln
        if let Some(src) = self.hide_timeout.borrow_mut().take() {
            src.remove();
        }

        let win = self.window.clone();
        let vol = self.vol_module.clone();
        let bright = self.bright_module.clone();
        let timeout_ref = self.hide_timeout.clone();

        let src = gtk4::glib::timeout_add_local_once(Duration::from_secs(2), move || {
            // Beide Module ausblenden
            vol.hide();
            bright.hide();

            // Nach Revealer-Animation das Fenster verstecken
            let win_c = win.clone();
            gtk4::glib::timeout_add_local_once(Duration::from_millis(300), move || {
                if !vol.is_active() && !bright.is_active() {
                    win_c.set_visible(false);
                }
            });

            *timeout_ref.borrow_mut() = None;
        });

        *self.hide_timeout.borrow_mut() = Some(src);
    }

    fn setup_subscriptions(&self, ctx: AppContext) {
        let win_vol = self.window.clone();
        let vol_mod = self.vol_module.clone();
        let last_vol = Rc::new(RefCell::new(None::<f64>));
        let last_mute = Rc::new(RefCell::new(None::<bool>));

        let manager_vol = self.clone();
        ctx.audio.subscribe(move |data| {
            let mut changed = false;
            if let Some(lv) = *last_vol.borrow() {
                if (lv - data.volume).abs() > 0.01 {
                    changed = true;
                }
            } else {
                changed = true;
            }

            if let Some(lm) = *last_mute.borrow() {
                if lm != data.is_muted {
                    changed = true;
                }
            } else {
                changed = true;
            }

            *last_vol.borrow_mut() = Some(data.volume);
            *last_mute.borrow_mut() = Some(data.is_muted);

            if changed {
                let icon_name = icons::volume_icon(data.volume, data.is_muted);

                if !win_vol.is_visible() {
                    win_vol.set_visible(true);
                }
                vol_mod.show(data.volume, icon_name);
                manager_vol.reset_hide_timeout();
            }
        });

        let win_bright = self.window.clone();
        let bright_mod = self.bright_module.clone();
        let last_bright = Rc::new(RefCell::new(None::<f64>));

        let manager_bright = self.clone();
        ctx.backlight.subscribe(move |data| {
            if !data.initialized {
                return;
            }

            let current_val = data.percentage / 100.0;
            let mut changed = false;

            if let Some(lb) = *last_bright.borrow() {
                if (lb - current_val).abs() > 0.001 {
                    changed = true;
                }
            } else {
                changed = true;
            }

            *last_bright.borrow_mut() = Some(current_val);

            if changed {
                let icon_name = if current_val < 0.33 {
                    "display-brightness-low-symbolic"
                } else if current_val < 0.66 {
                    "display-brightness-symbolic"
                } else {
                    "display-brightness-high-symbolic"
                };

                if !win_bright.is_visible() {
                    win_bright.set_visible(true);
                }
                bright_mod.show(current_val, icon_name);
                manager_bright.reset_hide_timeout();
            }
        });
    }
}

impl Clone for OsdManager {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
            vol_module: self.vol_module.clone(),
            bright_module: self.bright_module.clone(),
            hide_timeout: self.hide_timeout.clone(),
        }
    }
}

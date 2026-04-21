use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use axis_domain::models::audio::AudioStatus;
use axis_domain::models::brightness::BrightnessStatus;
use crate::presentation::audio::{AudioView, audio_icon};
use crate::presentation::brightness::BrightnessView;
use crate::presentation::presenter::View;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const OSD_AUTO_HIDE_MS: u64 = 300;
const OSD_SHOW_MS: u64 = 2000;

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
    inner_timeout: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
    last_volume: Cell<Option<f64>>,
    last_muted: Cell<Option<bool>>,
    last_brightness: Cell<Option<f64>>,
}

impl OsdManager {
    pub fn new(app: &libadwaita::Application) -> Self {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .build();

        window.init_layer_shell();
        window.add_css_class("osd-window");
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Right, 10);
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
        window.set_can_focus(false);

        let main_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        let bright_module = Rc::new(OsdModule::new("display-brightness-symbolic"));
        let vol_module = Rc::new(OsdModule::new("audio-volume-high-symbolic"));

        main_box.append(&bright_module.container);
        main_box.append(&vol_module.container);

        window.set_child(Some(&main_box));

        Self {
            window,
            vol_module,
            bright_module,
            hide_timeout: Rc::new(RefCell::new(None)),
            inner_timeout: Rc::new(RefCell::new(None)),
            last_volume: Cell::new(None),
            last_muted: Cell::new(None),
            last_brightness: Cell::new(None),
        }
    }

    fn cancel_timeouts(&self) {
        if let Some(src) = self.hide_timeout.borrow_mut().take() {
            src.remove();
        }
        if let Some(src) = self.inner_timeout.borrow_mut().take() {
            src.remove();
        }
    }

    fn reset_hide_timeout(&self) {
        self.cancel_timeouts();

        let win = self.window.clone();
        let vol = self.vol_module.clone();
        let bright = self.bright_module.clone();
        let timeout_ref = self.hide_timeout.clone();
        let inner_ref = self.inner_timeout.clone();

        let src = gtk4::glib::timeout_add_local_once(Duration::from_millis(OSD_SHOW_MS), move || {
            *timeout_ref.borrow_mut() = None;

            vol.hide();
            bright.hide();

            let win_c = win.clone();
            let vol_c = vol.clone();
            let bright_c = bright.clone();
            let inner_ref_c = inner_ref.clone();
            let inner_src =
                gtk4::glib::timeout_add_local_once(Duration::from_millis(OSD_AUTO_HIDE_MS), move || {
                    *inner_ref_c.borrow_mut() = None;
                    if !vol_c.is_active() && !bright_c.is_active() {
                        win_c.set_visible(false);
                    }
                });
            *inner_ref.borrow_mut() = Some(inner_src);
        });

        *self.hide_timeout.borrow_mut() = Some(src);
    }
}

impl View<AudioStatus> for OsdManager {
    fn render(&self, status: &AudioStatus) {
        let mut changed = false;

        if let Some(lv) = self.last_volume.get() {
            if (lv - status.volume).abs() > 0.01 {
                changed = true;
            }
        } else {
            changed = true;
        }

        if let Some(lm) = self.last_muted.get() {
            if lm != status.is_muted {
                changed = true;
            }
        } else {
            changed = true;
        }

        self.last_volume.set(Some(status.volume));
        self.last_muted.set(Some(status.is_muted));

        if changed {
            let icon_name = audio_icon(status);

            if !self.window.is_visible() {
                self.window.set_visible(true);
            }
            self.vol_module.show(status.volume, icon_name);
            self.reset_hide_timeout();
        }
    }
}

impl View<BrightnessStatus> for OsdManager {
    fn render(&self, status: &BrightnessStatus) {
        if !status.has_backlight {
            return;
        }

        let value = status.percentage / 100.0;
        let mut changed = false;

        if let Some(lb) = self.last_brightness.get() {
            if (lb - value).abs() > 0.001 {
                changed = true;
            }
        } else {
            changed = true;
        }

        self.last_brightness.set(Some(value));

        if changed {
            if !self.window.is_visible() {
                self.window.set_visible(true);
            }
            self.bright_module.show(value, "display-brightness-symbolic");
            self.reset_hide_timeout();
        }
    }
}

impl AudioView for OsdManager {
    fn on_volume_changed(&self, _f: Box<dyn Fn(f64) + 'static>) {}
    fn on_set_default_sink(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_default_source(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_sink_input_volume(&self, _f: Box<dyn Fn(u32, f64) + 'static>) {}
}

impl BrightnessView for OsdManager {
    fn on_brightness_changed(&self, _f: Box<dyn Fn(f64) + 'static>) {}
}

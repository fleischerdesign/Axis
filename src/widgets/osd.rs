use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use crate::app_context::AppContext;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

struct OsdModule {
    container: gtk4::Revealer,
    level_bar: gtk4::LevelBar,
    icon: gtk4::Image,
    hide_timeout: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
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
            hide_timeout: Rc::new(RefCell::new(None)),
        }
    }

    fn update(&self, value: f64, icon_name: &str, window: &gtk4::ApplicationWindow) {
        if let Some(src) = self.hide_timeout.borrow_mut().take() {
            src.remove();
        }

        self.level_bar.set_value(value);
        self.icon.set_icon_name(Some(icon_name));

        if !window.is_visible() {
            window.set_visible(true);
        }
        
        self.container.set_reveal_child(true);

        let rev_hide = self.container.clone();
        let hide_timeout_c = self.hide_timeout.clone();
        let win = window.clone();
        
        let src = gtk4::glib::timeout_add_local_once(Duration::from_secs(2), move || {
            rev_hide.set_reveal_child(false);
            
            // Wenn nach dem Ausfaden kein OSD mehr aktiv ist, verstecken wir das Fenster
            let win_c = win.clone();
            gtk4::glib::timeout_add_local_once(Duration::from_millis(300), move || {
                // Hier prüfen wir später, ob noch andere Module aktiv sind (via Manager)
                // Aber fürs Erste: Wenn dieser Revealer zu ist, ist das Fenster potenziell versteckbar
                if !rev_hide.reveals_child() {
                    let _ = win_c.is_visible(); // Nur um win_c zu nutzen
                }
            });
            *hide_timeout_c.borrow_mut() = None;
        });
        
        *self.hide_timeout.borrow_mut() = Some(src);
    }
}

pub struct OsdManager {
    window: gtk4::ApplicationWindow,
    vol_module: Rc<OsdModule>,
    bright_module: Rc<OsdModule>,
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
        window.set_margin(Edge::Right, 10); // Synchron mit QS/Launcher
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
        });

        manager.setup_subscriptions(ctx);

        manager
    }

    fn setup_subscriptions(&self, ctx: AppContext) {
        let win_vol = self.window.clone();
        let vol_mod = self.vol_module.clone();
        let last_vol = Rc::new(RefCell::new(None::<f64>));
        let last_mute = Rc::new(RefCell::new(None::<bool>));
        
        ctx.audio.subscribe(move |data| {
            let mut changed = false;
            if let Some(lv) = *last_vol.borrow() {
                if (lv - data.volume).abs() > 0.01 { changed = true; }
            } else { changed = true; }

            if let Some(lm) = *last_mute.borrow() {
                if lm != data.is_muted { changed = true; }
            } else { changed = true; }

            *last_vol.borrow_mut() = Some(data.volume);
            *last_mute.borrow_mut() = Some(data.is_muted);

            if changed {
                let icon_name = if data.is_muted || data.volume <= 0.01 { "audio-volume-muted-symbolic" }
                else if data.volume < 0.33 { "audio-volume-low-symbolic" }
                else if data.volume < 0.66 { "audio-volume-medium-symbolic" }
                else { "audio-volume-high-symbolic" };
                
                vol_mod.update(data.volume, icon_name, &win_vol);
            }
        });

        let win_bright = self.window.clone();
        let bright_mod = self.bright_module.clone();
        let last_bright = Rc::new(RefCell::new(None::<f64>));
        
        ctx.backlight.subscribe(move |data| {
            let current_val = data.percentage as f64 / 100.0;
            let mut changed = false;
            
            if let Some(lb) = *last_bright.borrow() {
                if (lb - current_val).abs() > 0.01 { changed = true; }
            } else { changed = true; }

            *last_bright.borrow_mut() = Some(current_val);

            if changed {
                let icon_name = if current_val < 0.33 { "display-brightness-symbolic" }
                else if current_val < 0.66 { "display-brightness-symbolic" }
                else { "display-brightness-symbolic" };
                
                bright_mod.update(current_val, icon_name, &win_bright);
            }
        });
        
        // Timer zum Verstecken des Fensters, wenn gar nichts mehr aktiv ist
        let win_final = self.window.clone();
        let vol_final = self.vol_module.clone();
        let bright_final = self.bright_module.clone();
        gtk4::glib::timeout_add_local(Duration::from_millis(500), move || {
            if win_final.is_visible() && !vol_final.container.reveals_child() && !bright_final.container.reveals_child() {
                win_final.set_visible(false);
            }
            gtk4::glib::ControlFlow::Continue
        });
    }
}

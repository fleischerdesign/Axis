use axis_domain::models::appearance::{AccentColor, ColorScheme};
use axis_domain::models::config::AppearanceConfig;
use crate::view::View;
use super::{generate_css, resolve_accent_hex, find_vibrant_accent};
use libadwaita as adw;
use gtk4::{glib, gdk_pixbuf};
use gdk_pixbuf::Pixbuf;
use std::cell::RefCell;
use std::rc::Rc;

pub struct GtkThemeService {
    provider: Rc<gtk4::CssProvider>,
    cached_accent_from_wallpaper: Rc<RefCell<Option<String>>>,
    last_wallpaper_path: Rc<RefCell<Option<String>>>,
    on_color_extracted: Rc<RefCell<Option<Box<dyn Fn(String) + 'static>>>>,
}

impl GtkThemeService {
    pub fn new(provider: Rc<gtk4::CssProvider>) -> Self {
        Self {
            provider,
            cached_accent_from_wallpaper: Rc::new(RefCell::new(None)),
            last_wallpaper_path: Rc::new(RefCell::new(None)),
            on_color_extracted: Rc::new(RefCell::new(None)),
        }
    }

    pub fn on_color_extracted<F: Fn(String) + 'static>(&self, f: F) {
        *self.on_color_extracted.borrow_mut() = Some(Box::new(f));
    }

    pub fn apply_color_scheme(scheme: &ColorScheme) {
        let adwaita_scheme = match scheme {
            ColorScheme::Dark => adw::ColorScheme::ForceDark,
            ColorScheme::Light => adw::ColorScheme::ForceLight,
            ColorScheme::System => adw::ColorScheme::PreferDark,
        };
        adw::StyleManager::default().set_color_scheme(adwaita_scheme);
    }

    fn write_system_accent_css(accent_hex: &str) {
        let config_dir = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            format!("{home}/.config")
        });

        let css = format!(
            "@define-color accent_bg_color {accent_hex};\n\
             @define-color accent_color {accent_hex};\n\
             @define-color accent_fg_color #ffffff;\n"
        );

        for gtk_ver in &["gtk-4.0", "gtk-3.0"] {
            let dir = format!("{config_dir}/{gtk_ver}");
            if let Err(e) = std::fs::create_dir_all(&dir) {
                log::warn!("[theme] Failed to create {dir}: {e}");
            }
            if let Err(e) = std::fs::write(format!("{dir}/gtk.css"), &css) {
                log::warn!("[theme] Failed to write {dir}/gtk.css: {e}");
            }
        }
    }

    fn apply_theme(&self, status: &AppearanceConfig, accent_hex: &str) {
        let css = generate_css(status, accent_hex);
        self.provider.load_from_string(&css);
        Self::apply_color_scheme(&status.color_scheme);
        Self::write_system_accent_css(accent_hex);
    }
}

impl View<AppearanceConfig> for GtkThemeService {
    fn render(&self, status: &AppearanceConfig) {
        match &status.accent_color {
            AccentColor::Auto => {
                if let Some(ref path) = status.wallpaper {
                    if self.last_wallpaper_path.borrow().as_ref() == Some(path) {
                        if let Some(cached) = self.cached_accent_from_wallpaper.borrow().clone() {
                            self.apply_theme(status, &cached);
                            if let Some(ref f) = *self.on_color_extracted.borrow() {
                                f(cached);
                            }
                            return;
                        }
                    }

                    let path_c = path.clone();
                    let provider = self.provider.clone();
                    let status_c = status.clone();
                    let cached_accent = self.cached_accent_from_wallpaper.clone();
                    let last_path = self.last_wallpaper_path.clone();
                    let on_extracted = self.on_color_extracted.clone();

                    glib::spawn_future_local(async move {
                        if let Some(hex) = extract_vibrant_color_stable(&path_c) {
                            *cached_accent.borrow_mut() = Some(hex.clone());
                            *last_path.borrow_mut() = Some(path_c.clone());

                            if let Some(ref f) = *on_extracted.borrow() {
                                f(hex.clone());
                            }

                            let css = generate_css(&status_c, &hex);
                            provider.load_from_string(&css);
                            Self::write_system_accent_css(&hex);
                        }
                    });
                }
            }
            other => {
                let hex = resolve_accent_hex(other);
                self.apply_theme(status, &hex);
            }
        }
    }
}

const WALLPAPER_THUMBNAIL_SIZE: i32 = 128;

fn extract_vibrant_color_stable(path: &str) -> Option<String> {
    let pixbuf = Pixbuf::from_file_at_size(path, WALLPAPER_THUMBNAIL_SIZE, WALLPAPER_THUMBNAIL_SIZE).ok()?;
    let pixels = unsafe { pixbuf.pixels() };

    find_vibrant_accent(
        pixels,
        pixbuf.width() as u32,
        pixbuf.height() as u32,
        pixbuf.n_channels() as u32,
        pixbuf.rowstride() as usize,
    )
}

use axis_domain::models::appearance::{AccentColor, AppearanceStatus, ColorScheme};
use crate::view::View;
use super::{generate_css, resolve_accent_hex, find_vibrant_accent};
use libadwaita::prelude::*;
use libadwaita as adw;
use gtk4::{glib, gdk_pixbuf};
use gdk_pixbuf::Pixbuf;
use std::cell::RefCell;
use std::rc::Rc;
use log::info;

pub struct GtkThemeService {
    provider: Rc<gtk4::CssProvider>,
    cached_accent_from_wallpaper: RefCell<Option<String>>,
    last_wallpaper_path: RefCell<Option<String>>,
}

impl GtkThemeService {
    pub fn new(provider: Rc<gtk4::CssProvider>) -> Self {
        Self {
            provider,
            cached_accent_from_wallpaper: RefCell::new(None),
            last_wallpaper_path: RefCell::new(None),
        }
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
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(format!("{dir}/gtk.css"), &css);
        }
    }

    fn apply_theme(&self, status: &AppearanceStatus, accent_hex: &str) {
        let css = generate_css(status, accent_hex);
        self.provider.load_from_string(&css);
        Self::apply_color_scheme(&status.color_scheme);
        Self::write_system_accent_css(accent_hex);
    }
}

impl View<AppearanceStatus> for GtkThemeService {
    fn render(&self, status: &AppearanceStatus) {
        match &status.accent_color {
            AccentColor::Auto => {
                if let Some(ref path) = status.wallpaper {
                    if self.last_wallpaper_path.borrow().as_ref() == Some(path) {
                        if let Some(cached) = self.cached_accent_from_wallpaper.borrow().clone() {
                            self.apply_theme(status, &cached);
                            return;
                        }
                    }

                    let path_c = path.clone();
                    let provider = self.provider.clone();
                    let status_c = status.clone();
                    
                    let cached_accent = self.cached_accent_from_wallpaper.as_ptr();
                    let last_path = self.last_wallpaper_path.as_ptr();

                    glib::spawn_future_local(async move {
                        // Schnellladung via Pixbuf
                        if let Some(hex) = extract_vibrant_color_stable(&path_c) {
                            unsafe {
                                (*cached_accent) = Some(hex.clone());
                                (*last_path) = Some(path_c.clone());
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
        
        Self::apply_color_scheme(&status.color_scheme);
    }
}

fn extract_vibrant_color_stable(path: &str) -> Option<String> {
    // 128x128 reicht völlig für ein stabiles Histogramm
    let pixbuf = Pixbuf::from_file_at_size(path, 128, 128).ok()?;
    let pixels = unsafe { pixbuf.pixels() };
    
    find_vibrant_accent(
        pixels, 
        pixbuf.width() as u32, 
        pixbuf.height() as u32, 
        pixbuf.n_channels() as u32, 
        pixbuf.rowstride() as usize
    )
}

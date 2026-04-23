use axis_domain::models::appearance::{AccentColor, AppearanceStatus, ColorScheme};
use crate::view::View;
use super::{generate_css, resolve_accent_hex, extract_accent_from_image};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use log::{info, warn};

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

    fn resolve_accent(&self, status: &AppearanceStatus) -> String {
        match &status.accent_color {
            AccentColor::Auto => {
                if let Some(ref path) = status.wallpaper {
                    let last_path = self.last_wallpaper_path.borrow();
                    if last_path.as_ref() != Some(path) {
                        drop(last_path);
                        if let Some(hex) = extract_accent_from_image(path) {
                            info!("[theme] Auto accent from wallpaper: {hex}");
                            *self.cached_accent_from_wallpaper.borrow_mut() = Some(hex.clone());
                            *self.last_wallpaper_path.borrow_mut() = Some(path.clone());
                            return hex;
                        }
                    } else if let Some(cached) = self.cached_accent_from_wallpaper.borrow().clone() {
                        return cached;
                    }
                }
                "#3584e4".to_string()
            }
            other => resolve_accent_hex(other),
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
            if let Err(e) = std::fs::create_dir_all(&dir) {
                warn!("[theme] Failed to create {dir}: {e}");
                continue;
            }
            if let Err(e) = std::fs::write(format!("{dir}/gtk.css"), &css) {
                warn!("[theme] Failed to write {dir}/gtk.css: {e}");
            }
        }
    }
}

impl View<AppearanceStatus> for GtkThemeService {
    fn render(&self, status: &AppearanceStatus) {
        let resolved_accent = self.resolve_accent(status);
        let css = generate_css(status, &resolved_accent);
        
        self.provider.load_from_string(&css);
        Self::apply_color_scheme(&status.color_scheme);
        Self::write_system_accent_css(&resolved_accent);

        info!(
            "[theme] Applied — accent: {}, scheme: {:?}, font: {:?}",
            resolved_accent,
            status.color_scheme,
            status.font
        );
    }
}

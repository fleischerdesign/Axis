use axis_domain::models::appearance::{AccentColor, AppearanceStatus, ColorScheme};
use log::{info, warn};
use std::cell::RefCell;
use std::rc::Rc;

use crate::presentation::presenter::View;

pub struct ThemeService {
    provider: Rc<gtk4::CssProvider>,
    cached_accent_from_wallpaper: RefCell<Option<String>>,
    last_wallpaper_path: RefCell<Option<String>>,
}

impl ThemeService {
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
                    if self.last_wallpaper_path.borrow().as_deref() != Some(path.as_str()) {
                        if let Some(hex) = extract_accent_color(path) {
                            info!("[theme] Auto accent from wallpaper: {hex}");
                            *self.cached_accent_from_wallpaper.borrow_mut() = Some(hex.clone());
                            *self.last_wallpaper_path.borrow_mut() = Some(path.clone());
                            return hex;
                        }
                    }
                    if let Some(cached) = self.cached_accent_from_wallpaper.borrow().clone() {
                        return cached;
                    }
                }
                "#3584e4".to_string()
            }
            other => other.hex_value().into_owned(),
        }
    }

    fn generate_css(&self, status: &AppearanceStatus) -> String {
        let accent = self.resolve_accent(status);
        let hover = lighten_hex(&accent, 0.15);

        let mut css = format!(
            "@define-color accent_bg_color {accent};\n\
             @define-color accent_fg_color #ffffff;\n\
             @define-color accent_hover_color {hover};\n"
        );

        if matches!(status.color_scheme, ColorScheme::Light) {
            css.push_str(LIGHT_THEME_COLORS);
        }

        if let Some(ref font) = status.font {
            css.push_str(&format!(
                "window {{ --font-family: \"{font}\"; }}\n\
                 window * {{ font-family: var(--font-family); }}\n"
            ));
        }

        css
    }

    fn apply_color_scheme(scheme: &ColorScheme) {
        let adwaita_scheme = match scheme {
            ColorScheme::Dark => libadwaita::ColorScheme::ForceDark,
            ColorScheme::Light => libadwaita::ColorScheme::ForceLight,
            ColorScheme::System => libadwaita::ColorScheme::PreferDark,
        };
        libadwaita::StyleManager::default().set_color_scheme(adwaita_scheme);
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

impl View<AppearanceStatus> for ThemeService {
    fn render(&self, status: &AppearanceStatus) {
        let css = self.generate_css(status);
        self.provider.load_from_string(&css);

        Self::apply_color_scheme(&status.color_scheme);

        let resolved_accent = self.resolve_accent(status);
        Self::write_system_accent_css(&resolved_accent);

        info!(
            "[theme] Applied — accent: {}, scheme: {:?}, font: {:?}",
            self.resolve_accent(status),
            status.color_scheme,
            status.font
        );
    }
}

static LIGHT_THEME_COLORS: &str = "\
@define-color window_bg_color #fafafa;
@define-color window_fg_color #1c1c1c;
@define-color card_bg_color #ebebeb;
@define-color border_color rgba(0, 0, 0, 0.08);
@define-color dim_label_color rgba(0, 0, 0, 0.5);
@define-color faint_label_color rgba(0, 0, 0, 0.3);
@define-color muted_label_color rgba(0, 0, 0, 0.4);
@define-color hover_bg_color rgba(0, 0, 0, 0.05);
@define-color hover_bg_color_strong rgba(0, 0, 0, 0.1);
@define-color body_text_color rgba(0, 0, 0, 0.75);
@define-color title_text_color rgba(0, 0, 0, 0.9);
@define-color section_label_color rgba(0, 0, 0, 0.6);
@define-color slider_trough_color #d0d0d0;
@define-color ws_dot_inactive_color rgba(0, 0, 0, 0.2);
@define-color ws_dot_hover_color rgba(0, 0, 0, 0.45);
";

fn extract_accent_color(path: &str) -> Option<String> {
    let img = image::open(path).ok()?;
    let resized = img.resize_exact(64, 64, image::imageops::FilterType::Nearest);
    let rgb = resized.to_rgb8();

    let (mut r_sum, mut g_sum, mut b_sum) = (0u64, 0u64, 0u64);
    let mut count = 0u64;

    for pixel in rgb.pixels() {
        r_sum += pixel[0] as u64;
        g_sum += pixel[1] as u64;
        b_sum += pixel[2] as u64;
        count += 1;
    }

    if count == 0 {
        return None;
    }

    let r = (r_sum / count) as u8;
    let g = (g_sum / count) as u8;
    let b = (b_sum / count) as u8;

    Some(format!("#{:02x}{:02x}{:02x}", r, g, b))
}

fn lighten_hex(hex: &str, amount: f64) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    let f = amount.clamp(0.0, 1.0);
    let r = (r as f64 + (255.0 - r as f64) * f) as u8;
    let g = (g as f64 + (255.0 - g as f64) * f) as u8;
    let b = (b as f64 + (255.0 - b as f64) * f) as u8;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (53, 132, 228)
    }
}

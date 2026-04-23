use axis_domain::models::appearance::{AccentColor, AppearanceStatus, ColorScheme};

pub mod gtk_service;

pub const LIGHT_THEME_COLORS: &str = "\
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

pub fn resolve_accent_hex(accent: &AccentColor) -> String {
    accent.hex_value().into_owned()
}

pub fn generate_css(status: &AppearanceStatus, resolved_accent: &str) -> String {
    let hover = lighten_hex(resolved_accent, 0.15);

    let mut css = format!(
        "@define-color accent_bg_color {resolved_accent};\n\
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

pub fn extract_accent_from_image(path: &str) -> Option<String> {
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

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

/// Analysiert Pixeldaten und findet die dominanteste, lebendige Farbe.
/// Nutzt Histogramm-Clustering für Stabilität gegen Bildrauschen.
pub fn find_vibrant_accent(pixels: &[u8], width: u32, height: u32, channels: u32, stride: usize) -> Option<String> {
    // 36 Bins für den Farbkreis (alle 10 Grad ein Bin)
    let mut bins = vec![0f32; 36];
    let mut bin_colors = vec![(0f32, 0f32, 0f32); 36];

    for y in 0..height {
        let row_offset = y as usize * stride;
        for x in 0..width {
            let p = row_offset + x as usize * channels as usize;
            let r = pixels[p];
            let g = pixels[p+1];
            let b = pixels[p+2];

            let (h, s, l) = rgb_to_hsl(r, g, b);

            // Nur Pixel mit Mindestsättigung und passender Helligkeit beachten
            if s > 0.15 && l > 0.15 && l < 0.85 {
                let bin_idx = (h * 35.0) as usize;
                // Gewichtung: Sättigung kombiniert mit "Helligkeits-Goldilocks-Zone"
                let weight = s * (1.0 - (l - 0.5).abs() * 2.0);
                
                bins[bin_idx] += weight;
                bin_colors[bin_idx].0 += r as f32 * weight;
                bin_colors[bin_idx].1 += g as f32 * weight;
                bin_colors[bin_idx].2 += b as f32 * weight;
            }
        }
    }

    // Finde den Bin mit der höchsten kumulierten Gewichtung
    let (winner_idx, &max_weight) = bins.iter().enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;

    if max_weight <= 0.0 { return None; }

    // Durchschnittsfarbe des Gewinner-Bins berechnen
    let final_r = (bin_colors[winner_idx].0 / max_weight) as u8;
    let final_g = (bin_colors[winner_idx].1 / max_weight) as u8;
    let final_b = (bin_colors[winner_idx].2 / max_weight) as u8;

    // Normalisierung für UI-Akzente
    let (h, mut s, mut l) = rgb_to_hsl(final_r, final_g, final_b);
    
    // Sättigung: Muss knallen (mind. 60%)
    s = s.max(0.60);
    // Helligkeit: Nicht zu finster, nicht zu grell (Bereich 0.45 - 0.65)
    l = l.clamp(0.45, 0.65);

    let (r, g, b) = hsl_to_rgb(h, s, l);
    Some(format!("#{:02x}{:02x}{:02x}", r, g, b))
}

// --- Hilfsfunktionen ---

pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min { return (0.0, 0.0, l); }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r { (g - b) / d + (if g < b { 6.0 } else { 0.0 }) }
            else if max == g { (b - r) / d + 2.0 }
            else { (r - g) / d + 4.0 };
    (h / 6.0, s, l)
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let (r, g, b) = if s == 0.0 { (l, l, l) } else {
        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;
        (hue_to_rgb(p, q, h + 1.0/3.0), hue_to_rgb(p, q, h), hue_to_rgb(p, q, h - 1.0/3.0))
    };
    ((r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8)
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0/6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0/2.0 { return q; }
    if t < 2.0/3.0 { return p + (q - p) * (2.0/3.0 - t) * 6.0; }
    p
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

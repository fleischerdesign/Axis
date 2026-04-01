use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;

pub struct WallpaperService {
    app: libadwaita::Application,
    windows: RefCell<Vec<gtk4::ApplicationWindow>>,
    texture: RefCell<Option<gtk4::gdk::Texture>>,
}

impl WallpaperService {
    pub fn new(app: &libadwaita::Application) -> Self {
        Self {
            app: app.clone(),
            windows: RefCell::new(Vec::new()),
            texture: RefCell::new(None),
        }
    }

    /// Show wallpaper on all monitors. Returns the texture (for lock screen).
    pub fn show(&self, path: &str) -> Option<gtk4::gdk::Texture> {
        info!("[wallpaper] Loading: {path}");

        let texture = gtk4::gdk::Texture::from_filename(path).map_err(|e| {
            error!("[wallpaper] Failed to load {path}: {e}");
        }).ok()?;

        self.close_all();

        let display = gtk4::gdk::Display::default().expect("No display available");
        let monitors = display.monitors();
        let mut windows = Vec::new();

        for i in 0..monitors.n_items() {
            let monitor = monitors.item(i).unwrap();
            let monitor = monitor.downcast_ref::<gtk4::gdk::Monitor>().unwrap();

            let picture = gtk4::Picture::for_paintable(&texture);
            picture.set_content_fit(gtk4::ContentFit::Cover);
            picture.set_hexpand(true);
            picture.set_vexpand(true);

            let window = gtk4::ApplicationWindow::builder()
                .application(&self.app)
                .title(format!("AXIS Wallpaper {}", i))
                .build();

            window.init_layer_shell();
            window.set_monitor(Some(monitor));
            window.set_layer(Layer::Background);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Right, true);
            window.set_exclusive_zone(-1);
            window.set_child(Some(&picture));
            window.present();

            windows.push(window);
        }

        info!("[wallpaper] Background windows created for {} monitors", monitors.n_items());
        *self.texture.borrow_mut() = Some(texture.clone());
        *self.windows.borrow_mut() = windows;
        Some(texture)
    }

    /// Remove all wallpaper windows.
    pub fn close_all(&self) {
        for window in self.windows.borrow().iter() {
            window.close();
        }
        self.windows.borrow_mut().clear();
        *self.texture.borrow_mut() = None;
    }

    /// Change wallpaper at runtime. Returns new texture for lock screen.
    pub fn set_wallpaper(&self, path: Option<&str>) -> Option<gtk4::gdk::Texture> {
        match path {
            Some(p) => self.show(p),
            None => {
                self.close_all();
                None
            }
        }
    }

    pub fn texture(&self) -> Option<gtk4::gdk::Texture> {
        self.texture.borrow().clone()
    }
}

/// Extract the average (mean) color from a wallpaper image.
/// Resizes to 64×64 for performance, then averages all pixels.
pub fn extract_accent_color(path: &str) -> Option<String> {
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

/// Lighten a hex color by a given amount (0.0–1.0).
/// Moves each channel toward 255 by the given fraction.
pub fn lighten_hex(hex: &str, amount: f64) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    let f = amount.clamp(0.0, 1.0);
    let r = (r as f64 + (255.0 - r as f64) * f) as u8;
    let g = (g as f64 + (255.0 - g as f64) * f) as u8;
    let b = (b as f64 + (255.0 - b as f64) * f) as u8;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
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

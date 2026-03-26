use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use log::{error, info};

pub struct WallpaperService;

impl WallpaperService {
    /// Loads the wallpaper and creates a Layer::Background window.
    /// Returns a downsampled texture suitable for a blurred lock screen background.
    pub fn show(app: &libadwaita::Application, path: &str) -> Option<gtk4::gdk::Texture> {
        info!("[wallpaper] Loading: {path}");

        let pixbuf = gdk_pixbuf::Pixbuf::from_file(path).map_err(|e| {
            error!("[wallpaper] Failed to load {path}: {e}");
        }).ok()?;

        // Desktop: full resolution
        let desktop_texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);

        let picture = gtk4::Picture::for_paintable(&desktop_texture);
        picture.set_content_fit(gtk4::ContentFit::Cover);
        picture.set_hexpand(true);
        picture.set_vexpand(true);

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("AXIS Wallpaper")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Background);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_exclusive_zone(-1);
        window.set_child(Some(&picture));
        window.present();

        info!("[wallpaper] Background window created");

        // Lock screen: downsampled for blur effect
        let w = pixbuf.width();
        let h = pixbuf.height();
        let target_w = 300_i32;
        let target_h = (target_w as f64 * h as f64 / w as f64).round() as i32;
        let small = pixbuf.scale_simple(target_w, target_h, gdk_pixbuf::InterpType::Bilinear)?;
        Some(gtk4::gdk::Texture::for_pixbuf(&small))
    }
}

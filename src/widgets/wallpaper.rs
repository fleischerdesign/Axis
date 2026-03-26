use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use log::{error, info};

pub struct WallpaperService;

impl WallpaperService {
    /// Creates a Layer::Background window displaying the wallpaper.
    /// Returns the full-resolution texture (can be used for a blurred lock screen).
    pub fn show(app: &libadwaita::Application, path: &str) -> Option<gtk4::gdk::Texture> {
        info!("[wallpaper] Loading: {path}");

        let texture = gtk4::gdk::Texture::from_filename(path).map_err(|e| {
            error!("[wallpaper] Failed to load {path}: {e}");
        }).ok()?;

        let picture = gtk4::Picture::for_paintable(&texture);
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
        Some(texture)
    }
}

use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;

use axis_presentation::View;
use axis_domain::models::appearance::AppearanceStatus;

type TextureCallback = dyn Fn(Option<gtk4::gdk::Texture>);

pub struct WallpaperService {
    app: libadwaita::Application,
    windows: RefCell<Vec<gtk4::ApplicationWindow>>,
    texture: RefCell<Option<gtk4::gdk::Texture>>,
    last_path: RefCell<Option<String>>,
    on_texture_change: RefCell<Option<Rc<TextureCallback>>>,
}

impl WallpaperService {
    pub fn new(app: &libadwaita::Application) -> Self {
        Self {
            app: app.clone(),
            windows: RefCell::new(Vec::new()),
            texture: RefCell::new(None),
            last_path: RefCell::new(None),
            on_texture_change: RefCell::new(None),
        }
    }

    pub fn on_texture_change(&self, callback: Rc<TextureCallback>) {
        if let Some(ref tex) = *self.texture.borrow() {
            callback(Some(tex.clone()));
        }
        *self.on_texture_change.borrow_mut() = Some(callback);
    }

    fn show(&self, path: &str) -> Option<gtk4::gdk::Texture> {
        info!("[wallpaper] Loading: {path}");

        let texture = gtk4::gdk::Texture::from_filename(path)
            .map_err(|e| {
                error!("[wallpaper] Failed to load {path}: {e}");
            })
            .ok()?;

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

        info!(
            "[wallpaper] Background windows created for {} monitors",
            monitors.n_items()
        );
        *self.texture.borrow_mut() = Some(texture.clone());

        if let Some(ref callback) = *self.on_texture_change.borrow() {
            callback(Some(texture.clone()));
        }

        *self.windows.borrow_mut() = windows;
        Some(texture)
    }

    fn close_all(&self) {
        for window in self.windows.borrow().iter() {
            window.close();
        }
        self.windows.borrow_mut().clear();
        *self.texture.borrow_mut() = None;
    }

    pub fn texture(&self) -> Option<gtk4::gdk::Texture> {
        self.texture.borrow().clone()
    }
}

impl View<AppearanceStatus> for WallpaperService {
    fn render(&self, status: &AppearanceStatus) {
        let new_path = status.wallpaper.as_deref();
        let last = self.last_path.borrow().clone();

        if new_path == last.as_deref() {
            return;
        }

        match new_path {
            Some(path) => {
                self.show(path);
                *self.last_path.borrow_mut() = Some(path.to_string());
            }
            None => {
                self.close_all();
                *self.last_path.borrow_mut() = None;
                if let Some(ref callback) = *self.on_texture_change.borrow() {
                    callback(None);
                }
            }
        }
    }
}

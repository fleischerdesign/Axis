use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use libadwaita::prelude::*;

use axis_domain::models::continuity::{ActiveDragPayload, ContinuityStatus};

pub struct DragOverlayWindow {
    window: gtk4::Window,
    container: Box,
    icon_label: Label,
    name_label: Label,
    size_label: Label,
    action_box: Box,
    btn_downloads: Button,
    btn_open: Button,
    btn_copy: Button,
    active_path: std::rc::Rc<std::cell::RefCell<Option<String>>>,
}

impl DragOverlayWindow {
    pub fn new() -> Self {
        let window = gtk4::Window::new();
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace("axis-drag-overlay");

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 24);
        window.set_margin(Edge::Right, 24);

        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        let container = Box::new(Orientation::Horizontal, 12);
        container.add_css_class("axis-drag-overlay");
        container.set_margin_start(16);
        container.set_margin_end(16);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let icon_label = Label::new(Some("📄"));
        icon_label.add_css_class("axis-drag-icon");

        let name_label = Label::new(Some("Datei"));
        name_label.add_css_class("axis-drag-name");

        let size_label = Label::new(Some("0 B"));
        size_label.add_css_class("axis-drag-size");

        let info_box = Box::new(Orientation::Vertical, 2);
        info_box.append(&name_label);
        info_box.append(&size_label);

        let action_box = Box::new(Orientation::Horizontal, 8);
        action_box.add_css_class("axis-drag-actions");

        let btn_downloads = Button::with_label("📥 In Downloads");
        btn_downloads.add_css_class("suggested-action");
        btn_downloads.add_css_class("pill");

        let btn_open = Button::with_label("📂 Im Ordner");
        btn_open.add_css_class("pill");

        let btn_copy = Button::with_label("📋 Kopieren");
        btn_copy.add_css_class("pill");

        action_box.append(&btn_downloads);
        action_box.append(&btn_open);
        action_box.append(&btn_copy);

        container.append(&icon_label);
        container.append(&info_box);
        container.append(&action_box);

        window.set_child(Some(&container));
        window.set_visible(false);

        let active_path = std::rc::Rc::new(std::cell::RefCell::new(None));

        let active_path_clone = active_path.clone();
        btn_downloads.connect_clicked(move |_| {
            if let Some(path_str) = active_path_clone.borrow().as_ref() {
                let src = std::path::Path::new(path_str);
                if let Some(dest_dir) = dirs::download_dir() {
                    let dest = dest_dir.join(src.file_name().unwrap_or_default());
                    let _ = std::fs::copy(src, dest);
                }
            }
        });

        let active_path_clone = active_path.clone();
        btn_open.connect_clicked(move |_| {
            if let Some(path_str) = active_path_clone.borrow().as_ref() {
                let _ = open::that(path_str);
            }
        });

        let active_path_clone = active_path.clone();
        btn_copy.connect_clicked(move |_| {
            if let Some(path_str) = active_path_clone.borrow().as_ref() {
                if let Some(display) = gdk4::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.set_text(&format!("file://{path_str}"));
                }
            }
        });

        Self {
            window,
            container,
            icon_label,
            name_label,
            size_label,
            action_box,
            btn_downloads,
            btn_open,
            btn_copy,
            active_path,
        }
    }

    pub fn update(&self, status: &ContinuityStatus) {
        if let Some(drag) = &status.active_drag {
            let icon = if drag.is_directory { "📁" } else { "📄" };
            self.icon_label.set_text(icon);
            self.name_label.set_text(&drag.name);

            let formatted_size = format_bytes(drag.size_bytes);
            self.size_label.set_text(&formatted_size);

            let tmp_path = format!("/tmp/axis_drag_drop/{}/{}", drag.transfer_id, drag.name);
            *self.active_path.borrow_mut() = Some(tmp_path);

            self.window.set_visible(true);
        } else {
            self.window.set_visible(false);
        }
    }

    pub fn window(&self) -> &gtk4::Window {
        &self.window
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

impl Default for DragOverlayWindow {
    fn default() -> Self {
        Self::new()
    }
}

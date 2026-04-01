use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use gtk4::gio;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;
use axis_core::services::settings::config::*;

pub struct AppearancePage;

impl SettingsPage for AppearancePage {
    fn id(&self) -> &'static str { "appearance" }
    fn title(&self) -> &'static str { "Appearance" }
    fn icon(&self) -> &'static str { "preferences-desktop-appearance-symbolic" }

    fn build(&self, proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        let config = proxy.config();
        let updating = Rc::new(Cell::new(false));

        // ── Theme ───────────────────────────────────────────────────────
        let theme_group = libadwaita::PreferencesGroup::builder()
            .title("Theme")
            .build();

        let theme_light = gtk4::CheckButton::with_label("Light");
        let theme_dark = gtk4::CheckButton::with_label("Dark");
        let theme_system = gtk4::CheckButton::with_label("System");
        theme_dark.set_group(Some(&theme_light));
        theme_system.set_group(Some(&theme_light));

        match config.appearance.theme {
            Theme::Light => theme_light.set_active(true),
            Theme::Dark => theme_dark.set_active(true),
            Theme::System => theme_system.set_active(true),
        }

        let make_theme_handler = |theme: Theme, proxy: Rc<SettingsProxy>, updating: Rc<Cell<bool>>| {
            move |btn: &gtk4::CheckButton| {
                if !btn.is_active() || updating.get() { return; }
                let mut cfg = proxy.config().appearance;
                cfg.theme = theme.clone();
                let p = proxy.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_appearance(&cfg).await;
                    p.update_cache_appearance(cfg);
                });
            }
        };

        theme_light.connect_toggled(make_theme_handler(Theme::Light, proxy.clone(), updating.clone()));
        theme_dark.connect_toggled(make_theme_handler(Theme::Dark, proxy.clone(), updating.clone()));
        theme_system.connect_toggled(make_theme_handler(Theme::System, proxy.clone(), updating.clone()));

        let theme_row = libadwaita::ActionRow::builder().title("Color Scheme").build();
        let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        btn_box.set_valign(gtk4::Align::Center);
        btn_box.append(&theme_light);
        btn_box.append(&theme_dark);
        btn_box.append(&theme_system);
        theme_row.add_suffix(&btn_box);
        theme_group.add(&theme_row);

        // ── Wallpaper ───────────────────────────────────────────────────
        let wallpaper_group = libadwaita::PreferencesGroup::builder()
            .title("Wallpaper")
            .description("Used as desktop background and lock screen backdrop")
            .build();

        let wallpaper_preview = gtk4::Image::builder()
            .pixel_size(48)
            .css_classes(["wallpaper-preview"])
            .build();
        update_wallpaper_preview(&wallpaper_preview, &config.appearance.wallpaper);

        let wallpaper_label = gtk4::Label::builder()
            .label(config.appearance.wallpaper.as_deref().unwrap_or("No wallpaper"))
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .hexpand(true)
            .css_classes(["dim-label"])
            .build();

        let wallpaper_choose_btn = gtk4::Button::builder()
            .label("Choose…")
            .css_classes(["flat"])
            .build();

        let wallpaper_clear_btn = gtk4::Button::builder()
            .icon_name("edit-clear-symbolic")
            .css_classes(["flat"])
            .tooltip_text("Clear wallpaper")
            .sensitive(config.appearance.wallpaper.is_some())
            .build();

        let wallpaper_row = libadwaita::ActionRow::builder().title("Background Image").build();
        let wallpaper_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        wallpaper_box.set_valign(gtk4::Align::Center);
        wallpaper_box.append(&wallpaper_preview);
        wallpaper_box.append(&wallpaper_label);
        wallpaper_box.append(&wallpaper_choose_btn);
        wallpaper_box.append(&wallpaper_clear_btn);
        wallpaper_row.add_suffix(&wallpaper_box);
        wallpaper_group.add(&wallpaper_row);

        // File dialog
        {
            let proxy_c = proxy.clone();
            let preview_c = wallpaper_preview.clone();
            let label_c = wallpaper_label.clone();
            let clear_c = wallpaper_clear_btn.clone();
            wallpaper_choose_btn.connect_clicked(move |btn| {
                let dialog = gtk4::FileDialog::builder()
                    .title("Choose Wallpaper")
                    .modal(true)
                    .build();

                let filter = gtk4::FileFilter::new();
                filter.set_name(Some("Images"));
                filter.add_mime_type("image/png");
                filter.add_mime_type("image/jpeg");
                filter.add_mime_type("image/webp");
                filter.add_mime_type("image/bmp");
                dialog.set_default_filter(Some(&filter));

                let p = proxy_c.clone();
                let preview_c = preview_c.clone();
                let label_c = label_c.clone();
                let clear_c = clear_c.clone();
                dialog.open(Some(btn.root().unwrap().downcast_ref::<gtk4::Window>().unwrap()), gio::Cancellable::NONE, move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            let path_str = path.to_string_lossy().to_string();
                            let mut cfg = p.config().appearance;
                            cfg.wallpaper = Some(path_str.clone());
                            let pp = p.clone();
                            gtk4::glib::spawn_future_local(async move {
                                let _ = pp.set_appearance(&cfg).await;
                                pp.update_cache_appearance(cfg);
                            });
                            update_wallpaper_preview(&preview_c, &Some(path_str.clone()));
                            label_c.set_label(&path_str);
                            clear_c.set_sensitive(true);
                        }
                    }
                });
            });
        }

        // Clear button
        {
            let proxy_c = proxy.clone();
            let preview_c = wallpaper_preview.clone();
            let label_c = wallpaper_label.clone();
            let clear_c = wallpaper_clear_btn.clone();
            wallpaper_clear_btn.connect_clicked(move |_| {
                let mut cfg = proxy_c.config().appearance;
                cfg.wallpaper = None;
                let p = proxy_c.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_appearance(&cfg).await;
                    p.update_cache_appearance(cfg);
                });
                update_wallpaper_preview(&preview_c, &None);
                label_c.set_label("No wallpaper");
                clear_c.set_sensitive(false);
            });
        }

        // ── Accent Color ────────────────────────────────────────────────
        let accent_group = libadwaita::PreferencesGroup::builder()
            .title("Accent Color")
            .build();

        let accent_grid = gtk4::FlowBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .homogeneous(true)
            .column_spacing(8)
            .row_spacing(8)
            .max_children_per_line(5)
            .min_children_per_line(5)
            .css_classes(["accent-grid"])
            .build();

        let active_accent = Rc::new(Cell::new(config.appearance.accent_color.clone()));

        for color in AccentColor::all_presets() {
            let btn = create_accent_button(color, active_accent.get() == *color);
            accent_grid.append(&btn);
        }

        // Wire click handlers after appending (parent FlowBoxChild must exist)
        {
            let accent_grid_c = accent_grid.clone();
            for (idx, color) in AccentColor::all_presets().iter().enumerate() {
                if let Some(child) = accent_grid_c.child_at_index(idx as i32) {
                    if let Some(box_widget) = child.child() {
                        if let Some(btn) = box_widget.downcast_ref::<gtk4::Button>() {
                            let grid_cc = accent_grid_c.clone();
                            let child_c = child.clone();
                            let gesture = gtk4::GestureClick::new();
                            gesture.connect_pressed(move |_, _, _, _| {
                                grid_cc.select_child(&child_c);
                            });
                            btn.add_controller(gesture);
                        }
                    }
                }
            }
        }

        // Select the active one
        let current_idx = AccentColor::all_presets()
            .iter()
            .position(|c| *c == config.appearance.accent_color)
            .unwrap_or(0);
        accent_grid.select_child(&accent_grid.child_at_index(current_idx as i32).unwrap());

        {
            let proxy_c = proxy.clone();
            let active_c = active_accent.clone();
            let updating_c = updating.clone();
            accent_grid.connect_selected_children_changed(move |fb| {
                if updating_c.get() { return; }
                if let Some(selected) = fb.selected_children().first() {
                    let idx = selected.index();
                    if let Some(color) = AccentColor::all_presets().get(idx as usize) {
                        if *color == active_c.get() { return; }
                        active_c.set(color.clone());
                        let mut cfg = proxy_c.config().appearance;
                        cfg.accent_color = color.clone();
                        let p = proxy_c.clone();
                        gtk4::glib::spawn_future_local(async move {
                            let _ = p.set_appearance(&cfg).await;
                            p.update_cache_appearance(cfg);
                        });
                    }
                }
            });
        }

        accent_group.add(&accent_grid);

        // ── Font ────────────────────────────────────────────────────────
        let font_group = libadwaita::PreferencesGroup::builder()
            .title("Font")
            .build();

        let font_dialog = gtk4::FontDialog::builder()
            .title("Select Font")
            .build();

        let font_btn = gtk4::FontDialogButton::builder()
            .dialog(&font_dialog)
            .build();

        if let Some(ref font) = config.appearance.font {
            font_btn.set_font_desc(&gtk4::pango::FontDescription::from_string(font));
        }

        let font_reset_btn = gtk4::Button::builder()
            .icon_name("edit-undo-symbolic")
            .css_classes(["flat"])
            .tooltip_text("Reset to system default")
            .sensitive(config.appearance.font.is_some())
            .build();

        let font_row = libadwaita::ActionRow::builder()
            .title("Typeface")
            .subtitle("Override the system font for the shell")
            .build();
        let font_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        font_box.set_valign(gtk4::Align::Center);
        font_box.append(&font_btn);
        font_box.append(&font_reset_btn);
        font_row.add_suffix(&font_box);
        font_group.add(&font_row);

        {
            let proxy_c = proxy.clone();
            let reset_c = font_reset_btn.clone();
            font_btn.connect_font_desc_notify(move |btn| {
                let desc = btn.font_desc();
                let font_str = desc.map(|d| d.to_string()).filter(|s| !s.is_empty());
                let mut cfg = proxy_c.config().appearance;
                cfg.font = font_str;
                reset_c.set_sensitive(cfg.font.is_some());
                let p = proxy_c.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_appearance(&cfg).await;
                    p.update_cache_appearance(cfg);
                });
            });
        }

        {
            let proxy_c = proxy.clone();
            let font_btn_c = font_btn.clone();
            let reset_c = font_reset_btn.clone();
            font_reset_btn.connect_clicked(move |_| {
                let mut cfg = proxy_c.config().appearance;
                cfg.font = None;
                font_btn_c.set_font_desc(&gtk4::pango::FontDescription::new());
                reset_c.set_sensitive(false);
                let p = proxy_c.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_appearance(&cfg).await;
                    p.update_cache_appearance(cfg);
                });
            });
        }

        // ── Page Assembly ───────────────────────────────────────────────
        let page = libadwaita::PreferencesPage::new();
        page.add(&theme_group);
        page.add(&wallpaper_group);
        page.add(&accent_group);
        page.add(&font_group);

        // Reactive: update UI on external config changes
        let wallpaper_preview_c = wallpaper_preview.clone();
        let wallpaper_label_c = wallpaper_label.clone();
        let wallpaper_clear_c = wallpaper_clear_btn.clone();
        let accent_grid_c = accent_grid.clone();
        let active_accent_c = active_accent.clone();
        let font_btn_c = font_btn.clone();
        let font_reset_c = font_reset_btn.clone();
        let updating_c = updating.clone();
        let proxy_c = proxy.clone();
        proxy.on_change(move || {
            let cfg = proxy_c.config();
            updating_c.set(true);

            update_wallpaper_preview(&wallpaper_preview_c, &cfg.appearance.wallpaper);
            wallpaper_label_c.set_label(cfg.appearance.wallpaper.as_deref().unwrap_or("No wallpaper"));
            wallpaper_clear_c.set_sensitive(cfg.appearance.wallpaper.is_some());

            if cfg.appearance.accent_color != active_accent_c.get() {
                active_accent_c.set(cfg.appearance.accent_color.clone());
                let idx = AccentColor::all_presets()
                    .iter()
                    .position(|c| *c == cfg.appearance.accent_color)
                    .unwrap_or(0);
                if let Some(child) = accent_grid_c.child_at_index(idx as i32) {
                    accent_grid_c.select_child(&child);
                }
            }

            if let Some(ref font) = cfg.appearance.font {
                font_btn_c.set_font_desc(&gtk4::pango::FontDescription::from_string(font));
            } else {
                font_btn_c.set_font_desc(&gtk4::pango::FontDescription::new());
            }
            font_reset_c.set_sensitive(cfg.appearance.font.is_some());

            updating_c.set(false);
        });

        page.into()
    }
}

fn create_accent_button(color: &AccentColor, active: bool) -> gtk4::Button {
    let css_class = match color {
        AccentColor::Blue   => "accent-blue",
        AccentColor::Teal   => "accent-teal",
        AccentColor::Green  => "accent-green",
        AccentColor::Yellow => "accent-yellow",
        AccentColor::Orange => "accent-orange",
        AccentColor::Red    => "accent-red",
        AccentColor::Pink   => "accent-pink",
        AccentColor::Purple => "accent-purple",
        AccentColor::Auto   => "accent-auto",
    };

    let mut classes = vec!["accent-swatch", css_class];
    if active {
        classes.push("active");
    }

    let icon = if *color == AccentColor::Auto {
        Some("palette-symbolic")
    } else {
        None
    };

    let btn = gtk4::Button::builder()
        .css_classes(classes.iter().copied().collect::<Vec<&str>>().as_slice())
        .width_request(36)
        .height_request(36)
        .valign(gtk4::Align::Center)
        .tooltip_text(match color {
            AccentColor::Auto => "Extract from wallpaper",
            _ => color.hex_value(),
        })
        .build();

    if let Some(icon_name) = icon {
        btn.set_icon_name(icon_name);
    }

    btn
}

fn update_wallpaper_preview(image: &gtk4::Image, path: &Option<String>) {
    if let Some(p) = path {
        if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file_at_scale(p, 48, 48, true) {
            let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
            image.set_paintable(Some(&texture));
            return;
        }
    }
    image.set_icon_name(Some("image-x-generic-symbolic"));
}

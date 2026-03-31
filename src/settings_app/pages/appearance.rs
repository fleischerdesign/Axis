use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;
use crate::config::*;

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

        // ── Style ───────────────────────────────────────────────────────
        let style_group = libadwaita::PreferencesGroup::builder()
            .title("Style")
            .build();

        // Opacity
        let opacity_row = libadwaita::ActionRow::builder().title("Bar Opacity").build();
        let opacity_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.05);
        opacity_slider.set_value(config.appearance.bar_opacity);
        opacity_slider.set_size_request(200, -1);
        opacity_slider.set_draw_value(true);
        opacity_slider.set_value_pos(gtk4::PositionType::Right);
        opacity_row.add_suffix(&opacity_slider);
        style_group.add(&opacity_row);

        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        let debounce: Rc<std::cell::RefCell<Option<gtk4::glib::SourceId>>> = Rc::new(std::cell::RefCell::new(None));
        opacity_slider.connect_value_changed(move |s| {
            if updating_c.get() { return; }
            if let Some(id) = debounce.borrow_mut().take() { id.remove(); }
            let val = s.value();
            let p = proxy_c.clone();
            let src = gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                let mut cfg = p.config().appearance;
                cfg.bar_opacity = val;
                let pp = p.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = pp.set_appearance(&cfg).await;
                    pp.update_cache_appearance(cfg);
                });
            });
            *debounce.borrow_mut() = Some(src);
        });

        // Corner Radius
        let radius_row = libadwaita::ActionRow::builder().title("Corner Radius").build();
        let radius_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 20.0, 1.0);
        radius_slider.set_value(config.appearance.corner_radius as f64);
        radius_slider.set_size_request(200, -1);
        radius_slider.set_draw_value(true);
        radius_slider.set_value_pos(gtk4::PositionType::Right);
        radius_row.add_suffix(&radius_slider);
        style_group.add(&radius_row);

        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        let debounce: Rc<std::cell::RefCell<Option<gtk4::glib::SourceId>>> = Rc::new(std::cell::RefCell::new(None));
        radius_slider.connect_value_changed(move |s| {
            if updating_c.get() { return; }
            if let Some(id) = debounce.borrow_mut().take() { id.remove(); }
            let val = s.value() as u32;
            let p = proxy_c.clone();
            let src = gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                let mut cfg = p.config().appearance;
                cfg.corner_radius = val;
                let pp = p.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = pp.set_appearance(&cfg).await;
                    pp.update_cache_appearance(cfg);
                });
            });
            *debounce.borrow_mut() = Some(src);
        });

        let page = libadwaita::PreferencesPage::new();
        page.add(&theme_group);
        page.add(&style_group);

        // Reactive: update sliders on external config changes
        let opacity_slider_c = opacity_slider.clone();
        let radius_slider_c = radius_slider.clone();
        let updating_c = updating.clone();
        let proxy_c = proxy.clone();
        proxy.on_change(move || {
            let cfg = proxy_c.config();
            updating_c.set(true);
            opacity_slider_c.set_value(cfg.appearance.bar_opacity);
            radius_slider_c.set_value(cfg.appearance.corner_radius as f64);
            updating_c.set(false);
        });

        page.into()
    }
}

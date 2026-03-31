use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;

pub struct NightlightPage;

impl SettingsPage for NightlightPage {
    fn id(&self) -> &'static str { "nightlight" }
    fn title(&self) -> &'static str { "Night Light" }
    fn icon(&self) -> &'static str { "night-light-symbolic" }

    fn build(&self, proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        let config = proxy.config();
        let updating = Rc::new(Cell::new(false));

        // ── Toggle ──────────────────────────────────────────────────────
        let toggle_group = libadwaita::PreferencesGroup::builder()
            .title("Night Light")
            .description("Reduce blue light for comfortable nighttime use")
            .build();

        let enable_row = libadwaita::SwitchRow::builder()
            .title("Enable")
            .build();
        enable_row.set_active(config.nightlight.enabled);

        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        enable_row.connect_active_notify(move |row| {
            if updating_c.get() { return; }
            let mut cfg = proxy_c.config().nightlight;
            cfg.enabled = row.is_active();
            let p = proxy_c.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.set_nightlight(&cfg).await;
                p.update_cache_nightlight(cfg);
            });
        });
        toggle_group.add(&enable_row);

        // ── Temperature ─────────────────────────────────────────────────
        let temp_group = libadwaita::PreferencesGroup::builder()
            .title("Color Temperature")
            .build();

        // Day Temperature
        let day_row = libadwaita::ActionRow::builder().title("Day Temperature").build();
        let day_label = gtk4::Label::new(Some(&format!("{} K", config.nightlight.temp_day)));
        let day_label_for_onchange = day_label.clone();
        day_label.add_css_class("dim-label");
        let day_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 2500.0, 6500.0, 100.0);
        day_slider.set_value(config.nightlight.temp_day as f64);
        day_slider.set_size_request(200, -1);
        day_slider.set_draw_value(false);
        day_row.add_suffix(&day_slider);
        day_row.add_suffix(&day_label);
        temp_group.add(&day_row);

        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        let debounce: Rc<std::cell::RefCell<Option<gtk4::glib::SourceId>>> = Rc::new(std::cell::RefCell::new(None));
        day_slider.connect_value_changed(move |s| {
            if updating_c.get() { return; }
            let val = s.value() as u32;
            day_label.set_text(&format!("{val} K"));
            if let Some(id) = debounce.borrow_mut().take() { id.remove(); }
            let p = proxy_c.clone();
            let src = gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                let mut cfg = p.config().nightlight;
                cfg.temp_day = val;
                let pp = p.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = pp.set_nightlight(&cfg).await;
                    pp.update_cache_nightlight(cfg);
                });
            });
            *debounce.borrow_mut() = Some(src);
        });

        // Night Temperature
        let night_row = libadwaita::ActionRow::builder().title("Night Temperature").build();
        let night_label = gtk4::Label::new(Some(&format!("{} K", config.nightlight.temp_night)));
        let night_label_for_onchange = night_label.clone();
        night_label.add_css_class("dim-label");
        let night_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 2500.0, 6500.0, 100.0);
        night_slider.set_value(config.nightlight.temp_night as f64);
        night_slider.set_size_request(200, -1);
        night_slider.set_draw_value(false);
        night_row.add_suffix(&night_slider);
        night_row.add_suffix(&night_label);
        temp_group.add(&night_row);

        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        let debounce: Rc<std::cell::RefCell<Option<gtk4::glib::SourceId>>> = Rc::new(std::cell::RefCell::new(None));
        night_slider.connect_value_changed(move |s| {
            if updating_c.get() { return; }
            let val = s.value() as u32;
            night_label.set_text(&format!("{val} K"));
            if let Some(id) = debounce.borrow_mut().take() { id.remove(); }
            let p = proxy_c.clone();
            let src = gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                let mut cfg = p.config().nightlight;
                cfg.temp_night = val;
                let pp = p.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = pp.set_nightlight(&cfg).await;
                    pp.update_cache_nightlight(cfg);
                });
            });
            *debounce.borrow_mut() = Some(src);
        });

        // ── Schedule ────────────────────────────────────────────────────
        let schedule_group = libadwaita::PreferencesGroup::builder()
            .title("Schedule")
            .description("Manual sunrise/sunset times")
            .build();

        let sunrise_row = libadwaita::EntryRow::builder()
            .title("Sunrise")
            .build();
        sunrise_row.set_text(&config.nightlight.sunrise);
        schedule_group.add(&sunrise_row);

        let sunset_row = libadwaita::EntryRow::builder()
            .title("Sunset")
            .build();
        sunset_row.set_text(&config.nightlight.sunset);
        schedule_group.add(&sunset_row);

        let apply_schedule: Rc<dyn Fn()> = Rc::new({
            let sunrise_row = sunrise_row.clone();
            let sunset_row = sunset_row.clone();
            let proxy = proxy.clone();
            let updating = updating.clone();
            move || {
                if updating.get() { return; }
                let mut cfg = proxy.config().nightlight;
                cfg.sunrise = sunrise_row.text().to_string();
                cfg.sunset = sunset_row.text().to_string();
                cfg.latitude.clear();
                cfg.longitude.clear();
                let p = proxy.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_nightlight(&cfg).await;
                    p.update_cache_nightlight(cfg);
                });
            }
        });

        let ac = apply_schedule.clone();
        sunrise_row.connect_activate(move |_| ac());
        let ac = apply_schedule.clone();
        let focus = gtk4::EventControllerFocus::new();
        focus.connect_leave(move |_| ac());
        sunrise_row.add_controller(focus);

        let ac = apply_schedule.clone();
        sunset_row.connect_activate(move |_| ac());
        let focus = gtk4::EventControllerFocus::new();
        let ac2 = apply_schedule.clone();
        focus.connect_leave(move |_| ac2());
        sunset_row.add_controller(focus);

        let page = libadwaita::PreferencesPage::new();
        page.add(&toggle_group);
        page.add(&temp_group);
        page.add(&schedule_group);

        // Reactive: update widgets on external config changes
        let enable_row_c = enable_row.clone();
        let day_slider_c = day_slider.clone();
        let night_slider_c = night_slider.clone();
        let day_label_c = day_label_for_onchange.clone();
            let night_label_c = night_label_for_onchange.clone();
        let sunrise_row_c = sunrise_row.clone();
        let sunset_row_c = sunset_row.clone();
        let updating_c = updating.clone();
        let proxy_c = proxy.clone();
        proxy.on_change(move || {
            let cfg = proxy_c.config();
            updating_c.set(true);
            enable_row_c.set_active(cfg.nightlight.enabled);
            day_slider_c.set_value(cfg.nightlight.temp_day as f64);
            day_label_c.set_text(&format!("{} K", cfg.nightlight.temp_day));
            night_slider_c.set_value(cfg.nightlight.temp_night as f64);
            night_label_c.set_text(&format!("{} K", cfg.nightlight.temp_night));
            sunrise_row_c.set_text(&cfg.nightlight.sunrise);
            sunset_row_c.set_text(&cfg.nightlight.sunset);
            updating_c.set(false);
        });

        page.into()
    }
}

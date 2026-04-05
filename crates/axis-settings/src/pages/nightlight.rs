use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;
use axis_core::services::nightlight::solar;

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
        let day_label_c = day_label.clone();
        let debounce: Rc<std::cell::RefCell<Option<gtk4::glib::SourceId>>> = Rc::new(std::cell::RefCell::new(None));
        day_slider.connect_value_changed(move |s| {
            if updating_c.get() { return; }
            let val = s.value() as u32;
            day_label_c.set_text(&format!("{val} K"));
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
        let night_label_c = night_label.clone();
        let debounce: Rc<std::cell::RefCell<Option<gtk4::glib::SourceId>>> = Rc::new(std::cell::RefCell::new(None));
        night_slider.connect_value_changed(move |s| {
            if updating_c.get() { return; }
            let val = s.value() as u32;
            night_label_c.set_text(&format!("{val} K"));
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
            .build();

        let auto_switch = libadwaita::SwitchRow::builder()
            .title("Automatic Schedule")
            .subtitle("Based on sunrise and sunset times")
            .build();
        auto_switch.set_active(config.nightlight.auto_schedule);
        schedule_group.add(&auto_switch);

        // Location rows (visible when auto_schedule is on)
        let location_group = libadwaita::PreferencesGroup::builder().build();

        let lat_row = libadwaita::EntryRow::builder()
            .title("Latitude")
            .build();
        lat_row.set_text(&config.nightlight.latitude);
        location_group.add(&lat_row);

        let lon_row = libadwaita::EntryRow::builder()
            .title("Longitude")
            .build();
        lon_row.set_text(&config.nightlight.longitude);
        location_group.add(&lon_row);

        let detect_btn = gtk4::Button::builder()
            .label("Standort erkennen")
            .css_classes(["flat"])
            .build();
        let detect_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        detect_box.append(&detect_btn);
        let detect_row = libadwaita::ActionRow::builder().build();
        detect_row.add_suffix(&detect_box);
        location_group.add(&detect_row);

        // Solar times info label
        let solar_info = gtk4::Label::builder()
            .label("")
            .wrap(true)
            .css_classes(["dim-label"])
            .build();
        solar_info.set_visible(config.nightlight.auto_schedule);
        let solar_row = libadwaita::ActionRow::builder().build();
        solar_row.add_suffix(&solar_info);
        location_group.add(&solar_row);

        // Manual schedule rows (visible when auto_schedule is off)
        let manual_group = libadwaita::PreferencesGroup::builder()
            .title("Manual Times")
            .build();

        let sunrise_row = libadwaita::EntryRow::builder()
            .title("Sunrise")
            .build();
        sunrise_row.set_text(&config.nightlight.sunrise);
        manual_group.add(&sunrise_row);

        let sunset_row = libadwaita::EntryRow::builder()
            .title("Sunset")
            .build();
        sunset_row.set_text(&config.nightlight.sunset);
        manual_group.add(&sunset_row);

        // Visibility based on auto_schedule
        location_group.set_visible(config.nightlight.auto_schedule);
        manual_group.set_visible(!config.nightlight.auto_schedule);

        // Auto schedule toggle
        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        let location_group_c = location_group.clone();
        let manual_group_c = manual_group.clone();
        let solar_info_c = solar_info.clone();
        auto_switch.connect_active_notify(move |row| {
            if updating_c.get() { return; }
            let mut cfg = proxy_c.config().nightlight;
            cfg.auto_schedule = row.is_active();
            location_group_c.set_visible(cfg.auto_schedule);
            manual_group_c.set_visible(!cfg.auto_schedule);
            solar_info_c.set_visible(cfg.auto_schedule);
            update_solar_label(&solar_info_c, &cfg);
            let p = proxy_c.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.set_nightlight(&cfg).await;
                p.update_cache_nightlight(cfg);
            });
        });

        // Detect location button
        let lat_row_c = lat_row.clone();
        let lon_row_c = lon_row.clone();
        let solar_info_c2 = solar_info.clone();
        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        detect_btn.connect_clicked(move |_| {
            let p = proxy_c.clone();
            let lr = lat_row_c.clone();
            let lonr = lon_row_c.clone();
            let si = solar_info_c2.clone();
            let up = updating_c.clone();
            gtk4::glib::spawn_future_local(async move {
                if up.get() { return; }
                let result = detect_location_via_geoclue().await;
                if let Ok((lat, lon)) = result {
                    lr.set_text(&format!("{:.4}", lat));
                    lonr.set_text(&format!("{:.4}", lon));
                    let mut cfg = p.config().nightlight;
                    cfg.latitude = format!("{:.4}", lat);
                    cfg.longitude = format!("{:.4}", lon);
                    update_solar_label(&si, &cfg);
                    let pp = p.clone();
                    let _ = pp.set_nightlight(&cfg).await;
                    pp.update_cache_nightlight(cfg);
                }
            });
        });

        // Lat/Lon entry apply
        let apply_location = Rc::new({
            let lat_row = lat_row.clone();
            let lon_row = lon_row.clone();
            let proxy = proxy.clone();
            let updating = updating.clone();
            let solar_info = solar_info.clone();
            move || {
                if updating.get() { return; }
                let mut cfg = proxy.config().nightlight;
                cfg.latitude = lat_row.text().to_string();
                cfg.longitude = lon_row.text().to_string();
                update_solar_label(&solar_info, &cfg);
                let p = proxy.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_nightlight(&cfg).await;
                    p.update_cache_nightlight(cfg);
                });
            }
        });

        let al = apply_location.clone();
        lat_row.connect_activate(move |_| al());
        let al = apply_location.clone();
        lat_row.add_controller({
            let f = gtk4::EventControllerFocus::new();
            let al2 = apply_location.clone();
            f.connect_leave(move |_| al2());
            f
        });

        let al = apply_location.clone();
        lon_row.connect_activate(move |_| al());
        let al2 = apply_location.clone();
        lon_row.add_controller({
            let f = gtk4::EventControllerFocus::new();
            f.connect_leave(move |_| al2());
            f
        });

        // Manual schedule apply
        let apply_schedule = Rc::new({
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
        sunrise_row.add_controller({
            let f = gtk4::EventControllerFocus::new();
            f.connect_leave(move |_| ac());
            f
        });

        let ac = apply_schedule.clone();
        sunset_row.connect_activate(move |_| ac());
        let ac2 = apply_schedule.clone();
        sunset_row.add_controller({
            let f = gtk4::EventControllerFocus::new();
            f.connect_leave(move |_| ac2());
            f
        });

        // Initial solar label
        update_solar_label(&solar_info, &config.nightlight);

        // ── Page Assembly ───────────────────────────────────────────────
        let page = libadwaita::PreferencesPage::new();
        page.add(&toggle_group);
        page.add(&temp_group);
        page.add(&schedule_group);
        page.add(&location_group);
        page.add(&manual_group);

        // Reactive updates
        let enable_row_c = enable_row.clone();
        let day_slider_c = day_slider.clone();
        let night_slider_c = night_slider.clone();
        let day_label_c = day_label_for_onchange.clone();
            let night_label_c = night_label_for_onchange.clone();
        let auto_switch_c = auto_switch.clone();
        let lat_row_c = lat_row.clone();
        let lon_row_c = lon_row.clone();
        let sunrise_row_c = sunrise_row.clone();
        let sunset_row_c = sunset_row.clone();
        let solar_info_c = solar_info.clone();
        let location_group_c = location_group.clone();
        let manual_group_c = manual_group.clone();
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
            auto_switch_c.set_active(cfg.nightlight.auto_schedule);
            lat_row_c.set_text(&cfg.nightlight.latitude);
            lon_row_c.set_text(&cfg.nightlight.longitude);
            sunrise_row_c.set_text(&cfg.nightlight.sunrise);
            sunset_row_c.set_text(&cfg.nightlight.sunset);
            location_group_c.set_visible(cfg.nightlight.auto_schedule);
            manual_group_c.set_visible(!cfg.nightlight.auto_schedule);
            update_solar_label(&solar_info_c, &cfg.nightlight);
            updating_c.set(false);
        });

        page.into()
    }
}

fn update_solar_label(label: &gtk4::Label, cfg: &axis_core::services::settings::config::NightlightConfig) {
    if cfg.auto_schedule && !cfg.latitude.is_empty() && !cfg.longitude.is_empty() {
        if let Ok(lat) = cfg.latitude.parse::<f64>() {
            if let Ok(lon) = cfg.longitude.parse::<f64>() {
                let now = gtk4::glib::DateTime::now_local().ok();
                if let Some(dt) = now {
                    if let Some(times) = solar::calculate_sunrise_sunset(lat, lon, dt.year(), dt.month() as u32, dt.day_of_month() as u32) {
                        label.set_text(&format!("Sonnenaufgang: {} · Sonnenuntergang: {}", times.sunrise, times.sunset));
                        label.set_visible(true);
                        return;
                    }
                }
            }
        }
    }
    label.set_visible(false);
}

async fn detect_location_via_geoclue() -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let conn = zbus::Connection::system().await?;
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.GeoClue2",
        "/org/freedesktop/GeoClue2/Manager",
        "org.freedesktop.GeoClue2.Manager",
    ).await?;

    let client_path: zbus::zvariant::OwnedObjectPath = proxy.call("GetClient", &("axis-settings",)).await?;

    let client_proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.GeoClue2",
        &client_path,
        "org.freedesktop.GeoClue2.Client",
    ).await?;

    client_proxy.call_method("Start", &()).await?;

    let location_path: zbus::zvariant::OwnedObjectPath = client_proxy.get_property("Location").await?;

    let loc_proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.GeoClue2",
        &location_path,
        "org.freedesktop.GeoClue2.Location",
    ).await?;

    let latitude: f64 = loc_proxy.get_property("Latitude").await?;
    let longitude: f64 = loc_proxy.get_property("Longitude").await?;

    let _: () = client_proxy.call("Stop", &()).await?;

    Ok((latitude, longitude))
}

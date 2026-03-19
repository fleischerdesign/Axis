use crate::app_context::AppContext;
use crate::services::audio::AudioCmd;
use crate::services::backlight::BacklightCmd;
use crate::services::bluetooth::BluetoothCmd;
use crate::services::network::NetworkCmd;
use crate::services::nightlight::NightlightCmd;
use crate::services::power::PowerData;
use crate::widgets::QsTile;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct MainPage {
    pub container: gtk4::Box,
    pub wifi_tile: Rc<QsTile>,
    pub eth_tile: Rc<QsTile>,
    pub bt_tile: Rc<QsTile>,
    pub nl_tile: Rc<QsTile>,
}

impl MainPage {
    pub fn new(
        ctx: AppContext,
        vol_icon_bar: gtk4::Image,
        open_wifi: impl Fn() + 'static,
        open_bt: impl Fn() + 'static,
        open_nl: impl Fn() + 'static,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 20);

        // --- TILES ---
        let grid = gtk4::Grid::new();
        grid.set_column_spacing(12);
        grid.set_row_spacing(12);
        grid.set_column_homogeneous(true);

        let wifi_tile = Rc::new(QsTile::new(
            "Wi-Fi",
            "network-wireless-signal-excellent-symbolic",
            true,
        ));
        let eth_tile = Rc::new(QsTile::new("Ethernet", "network-wired-symbolic", false));
        let bt_tile = Rc::new(QsTile::new("Bluetooth", "bluetooth-active-symbolic", true));
        let nl_tile = Rc::new(QsTile::new("Night Light", "night-light-symbolic", true));
        let airplane_tile = QsTile::new("Airplane", "airplane-mode-symbolic", false);

        grid.attach(&wifi_tile.container, 0, 0, 1, 1);
        grid.attach(&eth_tile.container, 1, 0, 1, 1);
        grid.attach(&bt_tile.container, 0, 1, 1, 1);
        grid.attach(&nl_tile.container, 1, 1, 1, 1);
        grid.attach(&airplane_tile.container, 0, 2, 1, 1);

        // --- VOLUME SLIDER ---
        let slider_overlay = gtk4::Overlay::new();
        slider_overlay.add_css_class("volume-slider");
        slider_overlay.set_hexpand(true);

        let vol_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        vol_slider.set_hexpand(true);

        let vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        vol_icon.set_pixel_size(22);
        vol_icon.set_margin_start(22);
        vol_icon.set_halign(gtk4::Align::Start);
        vol_icon.set_valign(gtk4::Align::Center);
        vol_icon.set_can_target(false);

        slider_overlay.set_child(Some(&vol_slider));
        slider_overlay.add_overlay(&vol_icon);

        // --- BRIGHTNESS SLIDER ---
        let brightness_overlay = gtk4::Overlay::new();
        brightness_overlay.add_css_class("volume-slider");
        brightness_overlay.set_hexpand(true);

        let brightness_slider =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 100.0, 1.0);
        brightness_slider.set_hexpand(true);

        let brightness_icon = gtk4::Image::from_icon_name("display-brightness-symbolic");
        brightness_icon.set_pixel_size(22);
        brightness_icon.set_margin_start(22);
        brightness_icon.set_halign(gtk4::Align::Start);
        brightness_icon.set_valign(gtk4::Align::Center);
        brightness_icon.set_can_target(false);

        brightness_overlay.set_child(Some(&brightness_slider));
        brightness_overlay.add_overlay(&brightness_icon);

        // Hide brightness initially until we know if there's a backlight
        brightness_overlay.set_visible(false);

        // --- BOTTOM ROW ---
        let bottom_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let battery_content = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let battery_icon = gtk4::Image::from_icon_name("battery-full-symbolic");
        let battery_label = gtk4::Label::new(Some("...%"));
        battery_content.append(&battery_icon);
        battery_content.append(&battery_label);
        let battery_btn = gtk4::Button::builder()
            .child(&battery_content)
            .css_classes(vec!["qs-battery-btn".to_string()])
            .build();

        let power_btn = Self::create_bottom_btn("system-shutdown-symbolic");
        let lock_btn = Self::create_bottom_btn("system-lock-screen-symbolic");
        let settings_btn = Self::create_bottom_btn("emblem-system-symbolic");
        let screenshot_btn = Self::create_bottom_btn("camera-photo-symbolic");

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        bottom_row.append(&battery_btn);
        bottom_row.append(&spacer);
        bottom_row.append(&screenshot_btn);
        bottom_row.append(&settings_btn);
        bottom_row.append(&lock_btn);
        bottom_row.append(&power_btn);

        container.append(&grid);
        container.append(&slider_overlay);
        container.append(&brightness_overlay);
        container.append(&bottom_row);

        // --- BUTTON ACTIONS ---

        // WiFi toggle: liest aktuellen State direkt aus dem Store
        let ctx_wifi = ctx.clone();
        let network_store = ctx.network.clone();
        wifi_tile.main_btn.connect_clicked(move |_| {
            let current = network_store.get().is_wifi_enabled;
            let _ = ctx_wifi
                .network_tx
                .send_blocking(NetworkCmd::ToggleWifi(!current));
        });
        wifi_tile
            .arrow_btn
            .as_ref()
            .unwrap()
            .connect_clicked(move |_| {
                open_wifi();
            });

        nl_tile
            .arrow_btn
            .as_ref()
            .unwrap()
            .connect_clicked(move |_| {
                open_nl();
            });

        // Bluetooth toggle: liest aktuellen State direkt aus dem Store
        let ctx_bt = ctx.clone();
        let bt_store = ctx.bluetooth.clone();
        bt_tile.main_btn.connect_clicked(move |_| {
            let current = bt_store.get().is_powered;
            let _ = ctx_bt
                .bluetooth_tx
                .send_blocking(BluetoothCmd::TogglePower(!current));
        });
        bt_tile
            .arrow_btn
            .as_ref()
            .unwrap()
            .connect_clicked(move |_| {
                open_bt();
            });

        // Night Light toggle
        let ctx_nl = ctx.clone();
        let night_store = ctx.nightlight.clone();
        nl_tile.main_btn.connect_clicked(move |_| {
            let current = night_store.get().enabled;
            let _ = ctx_nl
                .nightlight_tx
                .send_blocking(NightlightCmd::Toggle(!current));
        });

        // Network → Tile-States
        let wifi_tile_c = wifi_tile.clone();
        let eth_tile_c = eth_tile.clone();
        ctx.network.subscribe(move |data| {
            wifi_tile_c.set_active(data.is_wifi_enabled);
            eth_tile_c.set_active(data.is_ethernet_connected);

            if data.is_wifi_enabled {
                let icon_name = if !data.is_wifi_connected {
                    "network-wireless-offline-symbolic"
                } else if data.active_strength > 80 {
                    "network-wireless-signal-excellent-symbolic"
                } else if data.active_strength > 60 {
                    "network-wireless-signal-good-symbolic"
                } else if data.active_strength > 40 {
                    "network-wireless-signal-ok-symbolic"
                } else {
                    "network-wireless-signal-weak-symbolic"
                };
                wifi_tile_c.set_icon(icon_name);
            } else {
                wifi_tile_c.set_icon("network-wireless-disabled-symbolic");
            }
        });

        // Bluetooth → Tile-State
        let bt_tile_c = bt_tile.clone();
        ctx.bluetooth.subscribe(move |data| {
            bt_tile_c.set_active(data.is_powered);

            if data.is_powered {
                let any_connected = data.devices.iter().any(|d| d.is_connected);
                let icon_name = if any_connected {
                    "bluetooth-active-symbolic"
                } else {
                    "bluetooth-symbolic"
                };
                bt_tile_c.set_icon(icon_name);
            } else {
                bt_tile_c.set_icon("bluetooth-disabled-symbolic");
            }
        });

        // Nightlight → Tile-State
        let nl_tile_c = nl_tile.clone();
        ctx.nightlight.subscribe(move |data| {
            nl_tile_c.set_active(data.enabled);
            nl_tile_c.set_sensitive(data.available);
        });

        // Audio → Slider + Icon (mit Debounce gegen eigene Slider-Änderungen)
        let is_updating = Rc::new(std::cell::RefCell::new(false));
        let is_first_update = Rc::new(std::cell::RefCell::new(true));

        let vol_slider_c = vol_slider.clone();
        let vol_icon_c = vol_icon.clone();
        let vol_icon_bar_c = vol_icon_bar.clone();
        let is_updating_rx = is_updating.clone();
        let is_first_rx = is_first_update.clone();

        ctx.audio.subscribe(move |data| {
            let current = vol_slider_c.value();
            let diff = (current - data.volume).abs();
            let is_first = *is_first_rx.borrow();

            if is_first || diff > 0.01 {
                *is_first_rx.borrow_mut() = false;
                *is_updating_rx.borrow_mut() = true;
                vol_slider_c.set_value(data.volume);
                *is_updating_rx.borrow_mut() = false;

                let icon_name = if data.is_muted || data.volume <= 0.01 {
                    "audio-volume-muted-symbolic"
                } else if data.volume < 0.33 {
                    "audio-volume-low-symbolic"
                } else if data.volume < 0.66 {
                    "audio-volume-medium-symbolic"
                } else {
                    "audio-volume-high-symbolic"
                };
                vol_icon_c.set_icon_name(Some(icon_name));
                vol_icon_bar_c.set_icon_name(Some(icon_name));
            }
        });

        // Power → Batterie-Button
        // battery_btn per Referenz über den battery_content-Parent erreichbar machen
        let battery_btn_c = battery_btn.clone();
        ctx.power.subscribe(move |data| {
            Self::update_battery(&battery_label, &battery_icon, &battery_btn_c, data);
        });

        // Slider → AudioCmd (Gegenseite)
        let ctx_audio = ctx.clone();
        let is_updating_cmd = is_updating.clone();
        vol_slider.connect_value_changed(move |s| {
            if *is_updating_cmd.borrow() {
                return;
            }
            let val = s.value();
            let _ = ctx_audio.audio_tx.send_blocking(AudioCmd::SetVolume(val));
        });

        // --- BRIGHTNESS SLIDER REACTIVE ---
        let brightness_slider_c = brightness_slider.clone();
        let brightness_overlay_c = brightness_overlay.clone();
        let brightness_is_updating = Rc::new(std::cell::RefCell::new(false));

        // Brightness → Slider (vereinfachter Debounce)
        let brightness_is_updating_rx = brightness_is_updating.clone();

        ctx.backlight.subscribe(move |data| {
            if !data.initialized {
                return;
            }

            brightness_overlay_c.set_visible(data.has_backlight);

            let current = brightness_slider_c.value();
            let diff = (current - data.percentage).abs();

            if diff > 0.5 {
                *brightness_is_updating_rx.borrow_mut() = true;
                brightness_slider_c.set_value(data.percentage);
                *brightness_is_updating_rx.borrow_mut() = false;
            }
        });

        // Brightness Slider → BacklightCmd
        let ctx_backlight = ctx.clone();
        brightness_slider.connect_value_changed(move |s| {
            if *brightness_is_updating.borrow() {
                return;
            }
            let val = s.value();
            let _ = ctx_backlight
                .backlight_tx
                .send_blocking(BacklightCmd::SetBrightness(val));
        });

        Self {
            container,
            wifi_tile,
            eth_tile,
            bt_tile,
            nl_tile,
        }
    }

    fn create_bottom_btn(icon: &str) -> gtk4::Button {
        gtk4::Button::builder()
            .icon_name(icon)
            .css_classes(vec!["qs-bottom-btn".to_string()])
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build()
    }

    fn update_battery(
        label: &gtk4::Label,
        icon: &gtk4::Image,
        btn: &gtk4::Button,
        data: &PowerData,
    ) {
        if data.has_battery {
            // Echte Daten vorhanden: alles anzeigen und aktualisieren
            btn.set_visible(true);
            icon.set_visible(true);
            label.set_text(&format!("{:.0}%", data.battery_percentage));
            let icon_name = if data.is_charging {
                "battery-full-charging-symbolic"
            } else if data.battery_percentage < 10.0 {
                "battery-empty-symbolic"
            } else if data.battery_percentage < 30.0 {
                "battery-low-symbolic"
            } else if data.battery_percentage < 60.0 {
                "battery-good-symbolic"
            } else {
                "battery-full-symbolic"
            };
            icon.set_icon_name(Some(icon_name));
        } else {
            btn.set_visible(false);
        }
    }
}

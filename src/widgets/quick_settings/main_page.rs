use crate::app_context::AppContext;
use crate::services::audio::AudioCmd;
use crate::services::bluetooth::BluetoothCmd;
use crate::services::network::NetworkCmd;
use crate::widgets::quick_settings::components::QsTile;
use gtk4::glib::clone;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct MainPage {
    pub container: gtk4::Box,
    pub wifi_tile: Rc<QsTile>,
    pub eth_tile: Rc<QsTile>,
    pub bt_tile: Rc<QsTile>,
}

impl MainPage {
    pub fn new(
        ctx: AppContext,
        vol_icon_bar: gtk4::Image,
        open_wifi: impl Fn() + 'static,
        open_bt: impl Fn() + 'static,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 20);
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
        let night_tile = QsTile::new("Night Light", "night-light-symbolic", false);
        let airplane_tile = QsTile::new("Airplane", "airplane-mode-symbolic", false);

        grid.attach(&wifi_tile.container, 0, 0, 1, 1);
        grid.attach(&eth_tile.container, 1, 0, 1, 1);
        grid.attach(&bt_tile.container, 0, 1, 1, 1);
        grid.attach(&night_tile.container, 1, 1, 1, 1);
        grid.attach(&airplane_tile.container, 0, 2, 1, 1);

        // Volume Slider
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

        // Bottom Row
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

        bottom_row.append(&battery_btn);
        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        bottom_row.append(&spacer);
        bottom_row.append(&screenshot_btn);
        bottom_row.append(&settings_btn);
        bottom_row.append(&lock_btn);
        bottom_row.append(&power_btn);

        container.append(&grid);
        container.append(&slider_overlay);
        container.append(&bottom_row);

        // --- BUTTON ACTIONS ---
        let ctx_wifi = ctx.clone();
        wifi_tile.main_btn.connect_clicked(move |_| {
            let current = ctx_wifi
                .network_rx
                .try_recv()
                .unwrap_or_default()
                .is_wifi_enabled;
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

        let ctx_bt_click = ctx.clone();
        bt_tile.main_btn.connect_clicked(move |_| {
            let current = ctx_bt_click
                .bluetooth_rx
                .try_recv()
                .unwrap_or_default()
                .is_powered;
            let _ = ctx_bt_click
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

        // --- EVENT LOOPS ---

        // Network Loop (Tile State)
        let wifi_tile_c = wifi_tile.clone();
        let eth_tile_c = eth_tile.clone();
        let network_rx = ctx.network_rx.clone();
        gtk4::glib::spawn_future_local(clone!(
            #[strong]
            wifi_tile_c,
            #[strong]
            eth_tile_c,
            async move {
                while let Ok(data) = network_rx.recv().await {
                    wifi_tile_c.set_active(data.is_wifi_enabled);
                    eth_tile_c.set_active(data.is_ethernet_connected);
                }
            }
        ));

        // Bluetooth Loop (Tile State)
        let bt_tile_c = bt_tile.clone();
        let bluetooth_rx = ctx.bluetooth_rx.clone();
        gtk4::glib::spawn_future_local(clone!(
            #[strong]
            bt_tile_c,
            async move {
                while let Ok(data) = bluetooth_rx.recv().await {
                    bt_tile_c.set_active(data.is_powered);
                }
            }
        ));

        // Audio Loop
        let vol_slider_c = vol_slider.clone();
        let vol_icon_c = vol_icon.clone();
        let vol_icon_bar_c = vol_icon_bar.clone();
        let is_updating = Rc::new(RefCell::new(false));
        let is_updating_c = is_updating.clone();
        let last_sent = Rc::new(RefCell::new(0.0));
        let last_sent_c = last_sent.clone();
        let last_time = Rc::new(RefCell::new(std::time::Instant::now()));
        let last_time_c = last_time.clone();
        let audio_rx = ctx.audio_rx.clone();

        gtk4::glib::spawn_future_local(clone!(
            #[strong]
            vol_slider_c,
            #[strong]
            vol_icon_c,
            #[strong]
            vol_icon_bar_c,
            #[strong]
            is_updating_c,
            #[strong]
            last_sent_c,
            #[strong]
            last_time_c,
            async move {
                while let Ok(data) = audio_rx.recv().await {
                    let current = vol_slider_c.value();
                    let diff = (current - data.volume).abs();
                    let last = *last_sent_c.borrow();
                    let time = last_time_c.borrow().elapsed();
                    let is_grace = time < std::time::Duration::from_millis(600);

                    if (!is_grace && diff > 0.01) || (is_grace && (data.volume - last).abs() < 0.05) {
                        *is_updating_c.borrow_mut() = true;
                        vol_slider_c.set_value(data.volume);
                        *is_updating_c.borrow_mut() = false;

                        let icon_name = if data.is_muted || data.volume <= 0.01 { "audio-volume-muted-symbolic" }
                        else if data.volume < 0.33 { "audio-volume-low-symbolic" }
                        else if data.volume < 0.66 { "audio-volume-medium-symbolic" }
                        else { "audio-volume-high-symbolic" };
                        vol_icon_c.set_icon_name(Some(icon_name));
                        vol_icon_bar_c.set_icon_name(Some(icon_name));
                    }
                }
            }
        ));

        let ctx_audio = ctx.clone();
        let is_updating_tx = is_updating.clone();
        let last_sent_tx = last_sent.clone();
        let last_time_tx = last_time.clone();
        vol_slider.connect_value_changed(move |s| {
            if *is_updating_tx.borrow() {
                return;
            }
            *last_time_tx.borrow_mut() = std::time::Instant::now();
            let val = s.value();
            *last_sent_tx.borrow_mut() = val;
            let _ = ctx_audio.audio_tx.send_blocking(AudioCmd::SetVolume(val));
        });

        Self {
            container,
            wifi_tile,
            eth_tile,
            bt_tile,
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
}

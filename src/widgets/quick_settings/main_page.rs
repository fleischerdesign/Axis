use gtk4::prelude::*;
use crate::services::network::NetworkCmd;
use crate::services::bluetooth::BluetoothCmd;
use crate::services::audio::AudioCmd;
use crate::app_context::AppContext;
use crate::widgets::quick_settings::components::QsTile;
use std::rc::Rc;
use std::cell::RefCell;

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

        let wifi_tile = Rc::new(QsTile::new("Wi-Fi", "network-wireless-signal-excellent-symbolic", true));
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
        let battery_btn = gtk4::Button::builder().child(&battery_content).css_classes(vec!["qs-battery-btn".to_string()]).build();

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

        // Logic
        let ctx_c = ctx.clone();
        wifi_tile.main_btn.connect_clicked(move |_| {
            let _ = ctx_c.network_tx.unbounded_send(NetworkCmd::ToggleWifi(true));
        });
        wifi_tile.arrow_btn.as_ref().unwrap().connect_clicked(move |_| { open_wifi(); });
        bt_tile.arrow_btn.as_ref().unwrap().connect_clicked(move |_| { open_bt(); });
        
        let ctx_bt = ctx.clone();
        bt_tile.main_btn.connect_clicked(move |_| {
            let _ = ctx_bt.bluetooth_tx.unbounded_send(BluetoothCmd::TogglePower(true));
        });

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
        let mut audio_rx = ctx.audio_rx.clone();

        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut is_first = true;
            Self::update_audio_ui(&audio_rx.borrow(), &vol_slider_c, &vol_icon_c, &vol_icon_bar_c, &is_updating_c, &last_sent_c, &last_time_c, &mut is_first);

            while audio_rx.changed().await.is_ok() {
                Self::update_audio_ui(&audio_rx.borrow(), &vol_slider_c, &vol_icon_c, &vol_icon_bar_c, &is_updating_c, &last_sent_c, &last_time_c, &mut is_first);
            }
        });

        let ctx_audio = ctx.clone();
        let is_updating_tx = is_updating.clone();
        let last_sent_tx = last_sent.clone();
        let last_time_tx = last_time.clone();
        vol_slider.connect_value_changed(move |s| {
            if *is_updating_tx.borrow() { return; }
            *last_time_tx.borrow_mut() = std::time::Instant::now();
            let val = s.value();
            *last_sent_tx.borrow_mut() = val;
            let _ = ctx_audio.audio_tx.unbounded_send(AudioCmd::SetVolume(val));
        });

        // Power Loop
        let battery_btn_c = battery_btn.clone();
        let battery_icon_c = battery_icon.clone();
        let battery_label_c = battery_label.clone();
        let mut power_rx = ctx.power_rx.clone();

        gtk4::glib::MainContext::default().spawn_local(async move {
            Self::update_power_ui(&power_rx.borrow(), &battery_btn_c, &battery_icon_c, &battery_label_c);
            while power_rx.changed().await.is_ok() {
                Self::update_power_ui(&power_rx.borrow(), &battery_btn_c, &battery_icon_c, &battery_label_c);
            }
        });

        Self {
            container,
            wifi_tile, eth_tile, bt_tile
        }
    }

    fn update_audio_ui(
        data: &crate::services::audio::AudioData,
        slider: &gtk4::Scale,
        icon: &gtk4::Image,
        icon_bar: &gtk4::Image,
        is_updating: &Rc<RefCell<bool>>,
        last_sent: &Rc<RefCell<f64>>,
        last_time: &Rc<RefCell<std::time::Instant>>,
        is_first: &mut bool,
    ) {
        let current = slider.value();
        let diff = (current - data.volume).abs();
        let last = *last_sent.borrow();
        let time = last_time.borrow().elapsed();
        let is_grace = time < std::time::Duration::from_millis(600);

        if *is_first || (!is_grace && diff > 0.01) || (is_grace && (data.volume - last).abs() < 0.05) {
            *is_updating.borrow_mut() = true;
            slider.set_value(data.volume);
            *is_updating.borrow_mut() = false;
            *is_first = false;

            let icon_name = if data.is_muted || data.volume <= 0.01 { "audio-volume-muted-symbolic" }
            else if data.volume < 0.33 { "audio-volume-low-symbolic" }
            else if data.volume < 0.66 { "audio-volume-medium-symbolic" }
            else { "audio-volume-high-symbolic" };
            icon.set_icon_name(Some(icon_name));
            icon_bar.set_icon_name(Some(icon_name));
        }
    }

    fn update_power_ui(
        data: &crate::services::power::PowerData,
        btn: &gtk4::Button,
        icon: &gtk4::Image,
        label: &gtk4::Label,
    ) {
        btn.set_visible(data.has_battery);
        if data.has_battery {
            label.set_label(&format!("{}%", data.battery_percentage.round()));
            let icon_name = if data.is_charging { "battery-full-charging-symbolic" }
            else if data.battery_percentage < 10.0 { "battery-empty-symbolic" }
            else if data.battery_percentage < 30.0 { "battery-low-symbolic" }
            else if data.battery_percentage < 60.0 { "battery-good-symbolic" }
            else { "battery-full-symbolic" };
            icon.set_icon_name(Some(icon_name));
        }
    }

    fn create_bottom_btn(icon: &str) -> gtk4::Button {
        gtk4::Button::builder().icon_name(icon).css_classes(vec!["qs-bottom-btn".to_string()]).halign(gtk4::Align::Center).valign(gtk4::Align::Center).build()
    }
}

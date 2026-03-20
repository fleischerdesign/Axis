use crate::app_context::AppContext;
use crate::services::audio::AudioCmd;
use crate::services::backlight::BacklightCmd;
use crate::services::bluetooth::BluetoothCmd;
use crate::services::network::NetworkCmd;
use crate::services::nightlight::NightlightCmd;
use crate::widgets::components::icon_slider::IconSlider;
use crate::widgets::icons::bt::BtIcon;
use crate::widgets::icons::wifi::WifiIcon;
use crate::widgets::quick_settings::components::battery_button::BatteryButton;
use crate::widgets::quick_settings::components::power_actions::PowerActionStack;
use crate::widgets::{icons, ToggleTile};
use gtk4::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct MainPage {
    pub container: gtk4::Box,
    pub wifi_tile: Rc<ToggleTile>,
    pub eth_tile: Rc<ToggleTile>,
    pub bt_tile: Rc<ToggleTile>,
    pub nl_tile: Rc<ToggleTile>,
    power_actions: Rc<PowerActionStack>,
}

impl MainPage {
    pub fn new(
        ctx: AppContext,
        vol_icon_bar: gtk4::Image,
        open_wifi: impl Fn() + 'static,
        open_bt: impl Fn() + 'static,
        open_nl: impl Fn() + 'static,
        open_audio: impl Fn() + 'static,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 20);

        // --- TILES ---
        let grid = gtk4::Grid::new();
        grid.set_column_spacing(12);
        grid.set_row_spacing(12);
        grid.set_column_homogeneous(true);

        let wifi_tile = Rc::new(ToggleTile::new(
            "Wi-Fi",
            "network-wireless-signal-excellent-symbolic",
            true,
        ));
        let eth_tile = Rc::new(ToggleTile::new("Ethernet", "network-wired-symbolic", false));
        let bt_tile = Rc::new(ToggleTile::new("Bluetooth", "bluetooth-active-symbolic", true));
        let nl_tile = Rc::new(ToggleTile::new("Night Light", "night-light-symbolic", true));
        let airplane_tile = ToggleTile::new("Airplane", "airplane-mode-symbolic", false);

        grid.attach(&wifi_tile.container, 0, 0, 1, 1);
        grid.attach(&eth_tile.container, 1, 0, 1, 1);
        grid.attach(&bt_tile.container, 0, 1, 1, 1);
        grid.attach(&nl_tile.container, 1, 1, 1, 1);
        grid.attach(&airplane_tile.container, 0, 2, 1, 1);

        // --- VOLUME SLIDER ROW ---
        let vol_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        vol_row.add_css_class("volume-slider-row");

        let volume = IconSlider::new("audio-volume-high-symbolic", 0.0, 1.0, 0.01);
        volume.overlay.add_css_class("volume-slider");

        let vol_arrow = gtk4::Button::builder()
            .icon_name("go-next-symbolic")
            .css_classes(vec!["tile-arrow".to_string()])
            .build();

        vol_row.append(&volume.overlay);
        vol_row.append(&vol_arrow);

        vol_arrow.connect_clicked(move |_| {
            open_audio();
        });
        let vol_arrow_c = vol_arrow.clone();

        // --- BRIGHTNESS SLIDER ---
        let brightness = IconSlider::new("display-brightness-symbolic", 0.0, 100.0, 1.0);
        brightness.overlay.add_css_class("volume-slider");
        brightness.overlay.set_visible(false);

        // --- BOTTOM ROW ---
        let bottom_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let battery = BatteryButton::new(&ctx);

        let power_actions = Rc::new(PowerActionStack::new());

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        bottom_row.append(&battery.btn);
        bottom_row.append(&spacer);
        bottom_row.append(&power_actions.stack);

        container.append(&grid);
        container.append(&vol_row);
        container.append(&brightness.overlay);
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
        });

        let wifi_tile_c2 = wifi_tile.clone();
        WifiIcon::on_change(&ctx, move |name, _visible| {
            wifi_tile_c2.set_icon(name);
        });

        // Bluetooth → Tile-State
        let bt_tile_c = bt_tile.clone();
        ctx.bluetooth.subscribe(move |data| {
            bt_tile_c.set_active(data.is_powered);
        });

        let bt_tile_c2 = bt_tile.clone();
        BtIcon::on_change(&ctx, move |name, _visible| {
            bt_tile_c2.set_icon(name);
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

        let volume_c = volume.clone();
        let vol_icon_bar_c = vol_icon_bar.clone();
        let is_updating_rx = is_updating.clone();
        let is_first_rx = is_first_update.clone();

        let vol_arrow_sub = vol_arrow_c.clone();
        ctx.audio.subscribe(move |data| {
            let current = volume_c.slider.value();
            let diff = (current - data.volume).abs();
            let is_first = *is_first_rx.borrow();

            if is_first || diff > 0.01 {
                *is_first_rx.borrow_mut() = false;
                *is_updating_rx.borrow_mut() = true;
                volume_c.set_value(data.volume);
                *is_updating_rx.borrow_mut() = false;

                let icon_name = icons::volume_icon(data.volume, data.is_muted);
                volume_c.set_icon(icon_name);
                vol_icon_bar_c.set_icon_name(Some(icon_name));
            }
            Self::update_highlight_style(&volume_c.slider, data.volume, Some(&vol_arrow_sub));
        });

        // Initial style
        Self::update_highlight_style(&volume.slider, ctx.audio.get().volume, Some(&vol_arrow_c));

        // BatteryButton subscribes to power data internally

        // Slider → AudioCmd (Gegenseite)
        let ctx_audio = ctx.clone();
        let is_updating_cmd = is_updating.clone();
        let vol_arrow_chg = vol_arrow_c;
        volume.slider.connect_value_changed(move |s| {
            if *is_updating_cmd.borrow() {
                return;
            }
            let val = s.value();
            Self::update_highlight_style(s, val, Some(&vol_arrow_chg));
            let _ = ctx_audio.audio_tx.send_blocking(AudioCmd::SetVolume(val));
        });

        // --- BRIGHTNESS SLIDER REACTIVE ---
        let brightness_c = brightness.clone();
        let brightness_overlay_c = brightness.overlay.clone();
        let brightness_is_updating = Rc::new(std::cell::RefCell::new(false));

        // Brightness → Slider (vereinfachter Debounce)
        let brightness_is_updating_rx = brightness_is_updating.clone();

        ctx.backlight.subscribe(move |data| {
            if !data.initialized {
                return;
            }

            brightness_overlay_c.set_visible(data.has_backlight);

            let current = brightness_c.slider.value();
            let diff = (current - data.percentage).abs();

            if diff > 0.5 {
                *brightness_is_updating_rx.borrow_mut() = true;
                brightness_c.set_value(data.percentage);
                *brightness_is_updating_rx.borrow_mut() = false;
                Self::update_highlight_style(&brightness_c.slider, data.percentage, None);
            }
        });

        // Brightness Slider → BacklightCmd
        let ctx_backlight = ctx.clone();
        brightness.slider.connect_value_changed(move |s| {
            if *brightness_is_updating.borrow() {
                return;
            }
            let val = s.value();
            Self::update_highlight_style(s, val, None);
            let _ = ctx_backlight
                .backlight_tx
                .send_blocking(BacklightCmd::SetBrightness(val));
        });

        // PowerActionStack wires its own button actions internally

        Self {
            container,
            wifi_tile,
            eth_tile,
            bt_tile,
            nl_tile,
            power_actions,
        }
    }

    pub fn is_power_expanded(&self) -> bool {
        self.power_actions.is_power_expanded()
    }

    pub fn collapse_power_menu(&self) {
        self.power_actions.collapse_power_menu();
    }

    /// When volume < max, round the highlight's right edge so it looks clean.
    /// At 100%, remove the class so it flattens against the separator/arrow.
    fn update_highlight_style(scale: &gtk4::Scale, value: f64, arrow: Option<&gtk4::Button>) {
        if value < 0.99 {
            scale.add_css_class("highlight-partial");
            if let Some(btn) = arrow {
                btn.remove_css_class("max");
            }
        } else {
            scale.remove_css_class("highlight-partial");
            if let Some(btn) = arrow {
                btn.add_css_class("max");
            }
        }
    }
}

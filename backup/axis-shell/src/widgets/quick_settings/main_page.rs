use crate::app_context::AppContext;
use axis_core::services::airplane::AirplaneCmd;
use axis_core::services::audio::AudioCmd;
use axis_core::services::backlight::BacklightCmd;
use axis_core::services::bluetooth::BluetoothCmd;
use axis_core::services::continuity::ContinuityCmd;
use axis_core::services::dnd::DndCmd;
use axis_core::services::network::NetworkCmd;
use axis_core::services::nightlight::NightlightCmd;
use crate::widgets::components::debounced_slider::DebouncedSlider;
use crate::widgets::components::icon_slider::IconSlider;
use crate::widgets::icons;
use crate::widgets::icons::bluetooth::BluetoothIcon;
use crate::widgets::icons::wifi::WifiIcon;
use crate::widgets::quick_settings::components::battery_button::BatteryButton;
use crate::widgets::quick_settings::components::power_actions::PowerActionStack;
use crate::widgets::ToggleTile;
use gtk4::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct MainPage {
    pub container: gtk4::Box,
    pub wifi_tile: Rc<ToggleTile>,
    pub eth_tile: Rc<ToggleTile>,
    pub bluetooth_tile: Rc<ToggleTile>,
    pub nl_tile: Rc<ToggleTile>,
    pub airplane_tile: Rc<ToggleTile>,
    pub dnd_tile: Rc<ToggleTile>,
    power_actions: Rc<PowerActionStack>,
}

struct TileSet {
    grid: gtk4::Grid,
    wifi: Rc<ToggleTile>,
    eth: Rc<ToggleTile>,
    bluetooth: Rc<ToggleTile>,
    nightlight: Rc<ToggleTile>,
    airplane: Rc<ToggleTile>,
    dnd: Rc<ToggleTile>,
    kdeconnect: Rc<ToggleTile>,
    continuity: Rc<ToggleTile>,
}

impl MainPage {
    pub fn new(
        ctx: AppContext,
        vol_icon_bar: gtk4::Image,
        open_wifi: impl Fn() + 'static,
        open_bt: impl Fn() + 'static,
        open_nl: impl Fn() + 'static,
        open_audio: impl Fn() + 'static,
        open_kdeconnect: impl Fn() + 'static,
        open_continuity: impl Fn() + 'static,
        on_lock: Rc<dyn Fn()>,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 20);

        let tiles = Self::build_tile_grid();
        container.append(&tiles.grid);

        let vol_row = Self::build_volume_row(&ctx, &vol_icon_bar, open_audio);
        container.append(&vol_row.0);

        let brightness = Self::build_brightness_slider(&ctx);
        container.append(&brightness.icon_slider.overlay);

        let (bottom_row, power_actions) = Self::build_bottom_row(&ctx, on_lock);
        container.append(&bottom_row);

        Self::wire_tile_services(&tiles, &ctx, open_wifi, open_bt, open_nl, open_kdeconnect, open_continuity);
        Self::wire_dynamic_icons(&tiles, &ctx, &vol_icon_bar, &vol_row, &brightness);

        Self {
            container,
            wifi_tile: tiles.wifi,
            eth_tile: tiles.eth,
            bluetooth_tile: tiles.bluetooth,
            nl_tile: tiles.nightlight,
            airplane_tile: tiles.airplane,
            dnd_tile: tiles.dnd,
            power_actions,
        }
    }

    fn build_tile_grid() -> TileSet {
        let grid = gtk4::Grid::new();
        grid.set_column_spacing(12);
        grid.set_row_spacing(12);
        grid.set_column_homogeneous(true);

        let wifi = Rc::new(ToggleTile::new("Wi-Fi", "network-wireless-signal-excellent-symbolic", true));
        let eth = Rc::new(ToggleTile::new("Ethernet", "network-wired-symbolic", false));
        let bluetooth = Rc::new(ToggleTile::new("Bluetooth", "bluetooth-active-symbolic", true));
        let nightlight = Rc::new(ToggleTile::new("Night Light", "night-light-symbolic", true));
        let airplane = Rc::new(ToggleTile::new("Airplane", "airplane-mode-symbolic", false));
        let dnd = Rc::new(ToggleTile::new("DND", "preferences-system-notifications-symbolic", false));
        let kdeconnect = Rc::new(ToggleTile::new("KDE Connect", "phone-symbolic", true));
        let continuity = Rc::new(ToggleTile::new("Continuity", "input-mouse-symbolic", true));

        grid.attach(&wifi.container, 0, 0, 1, 1);
        grid.attach(&eth.container, 1, 0, 1, 1);
        grid.attach(&bluetooth.container, 0, 1, 1, 1);
        grid.attach(&nightlight.container, 1, 1, 1, 1);
        grid.attach(&airplane.container, 0, 2, 1, 1);
        grid.attach(&dnd.container, 1, 2, 1, 1);
        grid.attach(&kdeconnect.container, 0, 3, 1, 1);
        grid.attach(&continuity.container, 1, 3, 1, 1);

        TileSet { grid, wifi, eth, bluetooth, nightlight, airplane, dnd, kdeconnect, continuity }
    }

    fn build_volume_row(
        ctx: &AppContext,
        vol_icon_bar: &gtk4::Image,
        open_audio: impl Fn() + 'static,
    ) -> (gtk4::Box, DebouncedSlider<axis_core::services::audio::AudioData, AudioCmd>, gtk4::Button) {
        let vol_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        vol_row.add_css_class("volume-slider-row");

        let vol_icon_bar_c = vol_icon_bar.clone();
        let volume_slider = DebouncedSlider::new(
            "audio-volume-high-symbolic",
            0.0, 1.0, 0.01,
            &ctx.audio.store, &ctx.audio.tx,
            |d| d.volume,
            |v| AudioCmd::SetVolume(v),
            0.01,
            Some({
                let vol_icon_bar_c = vol_icon_bar_c.clone();
                move |slider: &IconSlider, data: &axis_core::services::audio::AudioData| {
                    let icon_name = icons::volume_icon(data.volume, data.is_muted);
                    slider.set_icon(icon_name);
                    vol_icon_bar_c.set_icon_name(Some(icon_name));
                }
            }),
            Some({
                let vol_icon_bar_c = vol_icon_bar_c.clone();
                move |slider: &IconSlider, val: f64| {
                    let icon_name = icons::volume_icon(val, false);
                    slider.set_icon(icon_name);
                    vol_icon_bar_c.set_icon_name(Some(icon_name));
                }
            }),
        );
        volume_slider.icon_slider.overlay.add_css_class("volume-slider");

        let vol_arrow = gtk4::Button::builder()
            .icon_name("go-next-symbolic")
            .css_classes(vec!["tile-arrow".to_string()])
            .build();

        vol_row.append(&volume_slider.icon_slider.overlay);
        vol_row.append(&vol_arrow);

        vol_arrow.connect_clicked(move |_| { open_audio(); });

        (vol_row, volume_slider, vol_arrow)
    }

    fn build_brightness_slider(ctx: &AppContext) -> DebouncedSlider<axis_core::services::backlight::BacklightData, BacklightCmd> {
        let brightness = DebouncedSlider::new(
            "display-brightness-symbolic",
            0.0, 100.0, 1.0,
            &ctx.backlight.store, &ctx.backlight.tx,
            |d| d.percentage,
            |v| BacklightCmd::SetBrightness(v),
            0.5,
            None::<fn(&IconSlider, &_)>,
            None::<fn(&IconSlider, f64)>,
        );
        brightness.icon_slider.overlay.add_css_class("volume-slider");
        brightness.icon_slider.overlay.set_visible(false);
        brightness
    }

    fn build_bottom_row(
        ctx: &AppContext,
        on_lock: Rc<dyn Fn()>,
    ) -> (gtk4::Box, Rc<PowerActionStack>) {
        let bottom_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let battery = BatteryButton::new(ctx);
        let power_actions = Rc::new(PowerActionStack::new(on_lock));
        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        bottom_row.append(&battery.btn);
        bottom_row.append(&spacer);
        bottom_row.append(&power_actions.stack);

        (bottom_row, power_actions)
    }

    fn wire_tile_services(
        tiles: &TileSet,
        ctx: &AppContext,
        open_wifi: impl Fn() + 'static,
        open_bt: impl Fn() + 'static,
        open_nl: impl Fn() + 'static,
        open_kdeconnect: impl Fn() + 'static,
        open_continuity: impl Fn() + 'static,
    ) {
        ToggleTile::wire_service(&tiles.wifi, &ctx.network,
            |on| NetworkCmd::ToggleWifi(on),
            |d| d.is_wifi_enabled,
            open_wifi,
            |_, _| {},
        );

        ToggleTile::wire_service(&tiles.bluetooth, &ctx.bluetooth,
            |on| BluetoothCmd::TogglePower(on),
            |d| d.is_powered,
            open_bt,
            |_, _| {},
        );

        ToggleTile::wire_service(&tiles.nightlight, &ctx.nightlight,
            |on| NightlightCmd::Toggle(on),
            |d| d.enabled,
            open_nl,
            |tile, data| { tile.set_sensitive(data.available); },
        );

        ToggleTile::wire_service(&tiles.dnd, &ctx.dnd,
            |on| DndCmd::Toggle(on),
            |d| d.enabled,
            || {},
            |_, _| {},
        );

        ToggleTile::wire_service(&tiles.airplane, &ctx.airplane,
            |on| AirplaneCmd::Toggle(on),
            |d| d.enabled,
            || {},
            |_, _| {},
        );

        tiles.kdeconnect.arrow_btn.as_ref().unwrap().connect_clicked(move |_| {
            open_kdeconnect();
        });
        let kdeconnect_tile_c = tiles.kdeconnect.clone();
        ctx.kdeconnect.subscribe(move |data| {
            let has_paired = data.devices.iter().any(|d| d.is_paired && d.is_reachable);
            kdeconnect_tile_c.set_active(has_paired);
            kdeconnect_tile_c.set_sensitive(data.available);
        });

        tiles.continuity.arrow_btn.as_ref().unwrap().connect_clicked(move |_| {
            open_continuity();
        });
        ToggleTile::wire_service(&tiles.continuity, &ctx.continuity,
            |on| ContinuityCmd::SetEnabled(on),
            |d| d.enabled,
            || {},
            |tile, data| {
                tile.set_active(data.enabled);
            },
        );

        let eth_tile_c = tiles.eth.clone();
        ctx.network.subscribe(move |data| {
            eth_tile_c.set_active(data.is_ethernet_connected);
        });
    }

    fn wire_dynamic_icons(
        tiles: &TileSet,
        ctx: &AppContext,
        vol_icon_bar: &gtk4::Image,
        vol_row: &(gtk4::Box, DebouncedSlider<axis_core::services::audio::AudioData, AudioCmd>, gtk4::Button),
        brightness: &DebouncedSlider<axis_core::services::backlight::BacklightData, BacklightCmd>,
    ) {
        let wifi_tile_c = tiles.wifi.clone();
        WifiIcon::on_change(ctx, move |name, _visible| {
            wifi_tile_c.set_icon(name);
        });

        let bluetooth_tile_c = tiles.bluetooth.clone();
        BluetoothIcon::on_change(ctx, move |name, _visible| {
            bluetooth_tile_c.set_icon(name);
        });

        // Volume highlight style
        let vol_arrow_sub = vol_row.2.clone();
        let vol_arrow_chg = vol_row.2.clone();
        let vol_arrow_init = vol_row.2.clone();
        let vol_slider = vol_row.1.clone();
        let vol_slider_sub = vol_slider.clone();
        let vol_slider_init = vol_slider.clone();
        ctx.audio.subscribe(move |data| {
            Self::update_highlight_style(
                &vol_slider_sub.icon_slider.slider,
                data.volume,
                Some(&vol_arrow_sub),
            );
        });
        vol_row.1.icon_slider.slider.connect_value_changed(move |s| {
            Self::update_highlight_style(s, s.value(), Some(&vol_arrow_chg));
        });
        Self::update_highlight_style(
            &vol_slider_init.icon_slider.slider,
            ctx.audio.get().volume,
            Some(&vol_arrow_init),
        );

        // Brightness visibility
        let brightness_overlay_c = brightness.icon_slider.overlay.clone();
        ctx.backlight.subscribe(move |data| {
            if data.initialized {
                brightness_overlay_c.set_visible(data.has_backlight);
            }
        });
        brightness.icon_slider.slider.connect_value_changed(move |s| {
            Self::update_highlight_style(s, s.value(), None);
        });
    }

    pub fn is_power_expanded(&self) -> bool {
        self.power_actions.is_power_expanded()
    }

    pub fn collapse_power_menu(&self) {
        self.power_actions.collapse_power_menu();
    }

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

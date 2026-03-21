use crate::app_context::AppContext;
use crate::services::audio::AudioCmd;
use crate::services::backlight::BacklightCmd;
use crate::services::bluetooth::BluetoothCmd;
use crate::services::dnd::DndCmd;
use crate::services::network::NetworkCmd;
use crate::services::nightlight::NightlightCmd;
use crate::widgets::components::debounced_slider::DebouncedSlider;
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
    pub dnd_tile: Rc<ToggleTile>,
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
        let bt_tile = Rc::new(ToggleTile::new(
            "Bluetooth",
            "bluetooth-active-symbolic",
            true,
        ));
        let nl_tile = Rc::new(ToggleTile::new("Night Light", "night-light-symbolic", true));
        let airplane_tile = ToggleTile::new("Airplane", "airplane-mode-symbolic", false);
        let dnd_tile = Rc::new(ToggleTile::new(
            "DND",
            "preferences-system-notifications-symbolic",
            false,
        ));

        grid.attach(&wifi_tile.container, 0, 0, 1, 1);
        grid.attach(&eth_tile.container, 1, 0, 1, 1);
        grid.attach(&bt_tile.container, 0, 1, 1, 1);
        grid.attach(&nl_tile.container, 1, 1, 1, 1);
        grid.attach(&airplane_tile.container, 0, 2, 1, 1);
        grid.attach(&dnd_tile.container, 1, 2, 1, 1);

        // --- VOLUME SLIDER ROW ---
        let vol_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        vol_row.add_css_class("volume-slider-row");

        let volume_slider = DebouncedSlider::new(
            "audio-volume-high-symbolic",
            0.0,
            1.0,
            0.01,
            &ctx.audio.store,
            &ctx.audio.tx,
            |d| d.volume,
            |v| AudioCmd::SetVolume(v),
            0.01,
            Some({
                let vol_icon_bar_c = vol_icon_bar.clone();
                move |slider: &IconSlider, data: &crate::services::audio::AudioData| {
                    let icon_name = icons::volume_icon(data.volume, data.is_muted);
                    slider.set_icon(icon_name);
                    vol_icon_bar_c.set_icon_name(Some(icon_name));
                }
            }),
        );
        volume_slider
            .icon_slider
            .overlay
            .add_css_class("volume-slider");

        let vol_arrow = gtk4::Button::builder()
            .icon_name("go-next-symbolic")
            .css_classes(vec!["tile-arrow".to_string()])
            .build();

        vol_row.append(&volume_slider.icon_slider.overlay);
        vol_row.append(&vol_arrow);

        vol_arrow.connect_clicked(move |_| {
            open_audio();
        });
        let vol_arrow_c = vol_arrow.clone();

        // --- BRIGHTNESS SLIDER ---
        let brightness_slider = DebouncedSlider::new(
            "display-brightness-symbolic",
            0.0,
            100.0,
            1.0,
            &ctx.backlight.store,
            &ctx.backlight.tx,
            |d| d.percentage,
            |v| BacklightCmd::SetBrightness(v),
            0.5,
            None::<fn(&IconSlider, &_)>,
        );
        brightness_slider
            .icon_slider
            .overlay
            .add_css_class("volume-slider");
        brightness_slider.icon_slider.overlay.set_visible(false);

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
        container.append(&brightness_slider.icon_slider.overlay);
        container.append(&bottom_row);

        // --- BUTTON ACTIONS ---

        // WiFi toggle: liest aktuellen State direkt aus dem Store
        let ctx_wifi = ctx.clone();
        let network_store = ctx.network.clone();
        wifi_tile.main_btn.connect_clicked(move |_| {
            let current = network_store.get().is_wifi_enabled;
            let _ = ctx_wifi
                .network
                .tx
                .try_send(NetworkCmd::ToggleWifi(!current));
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
                .bluetooth
                .tx
                .try_send(BluetoothCmd::TogglePower(!current));
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
                .nightlight
                .tx
                .try_send(NightlightCmd::Toggle(!current));
        });

        // DND toggle
        let ctx_dnd = ctx.clone();
        let dnd_store = ctx.dnd.clone();
        dnd_tile.main_btn.connect_clicked(move |_| {
            let current = dnd_store.get().enabled;
            let _ = ctx_dnd.dnd.tx.try_send(DndCmd::Toggle(!current));
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

        // DND → Tile-State
        let dnd_tile_c = dnd_tile.clone();
        ctx.dnd.subscribe(move |data| {
            dnd_tile_c.set_active(data.enabled);
        });

        // Volume highlight style (on subscribe + on user change)
        let vol_arrow_sub = vol_arrow_c.clone();
        let volume_slider_c = volume_slider.icon_slider.clone();
        ctx.audio.subscribe(move |data| {
            Self::update_highlight_style(
                &volume_slider_c.slider,
                data.volume,
                Some(&vol_arrow_sub),
            );
        });
        let vol_arrow_chg = vol_arrow_c.clone();
        volume_slider
            .icon_slider
            .slider
            .connect_value_changed(move |s| {
                Self::update_highlight_style(s, s.value(), Some(&vol_arrow_chg));
            });

        // Initial style
        Self::update_highlight_style(
            &volume_slider.icon_slider.slider,
            ctx.audio.get().volume,
            Some(&vol_arrow_c),
        );

        // BatteryButton subscribes to power data internally

        // Brightness visibility + highlight style
        let brightness_overlay_c = brightness_slider.icon_slider.overlay.clone();
        ctx.backlight.subscribe(move |data| {
            if data.initialized {
                brightness_overlay_c.set_visible(data.has_backlight);
            }
        });
        brightness_slider
            .icon_slider
            .slider
            .connect_value_changed(move |s| {
                Self::update_highlight_style(s, s.value(), None);
            });

        // PowerActionStack wires its own button actions internally

        Self {
            container,
            wifi_tile,
            eth_tile,
            bt_tile,
            nl_tile,
            dnd_tile,
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

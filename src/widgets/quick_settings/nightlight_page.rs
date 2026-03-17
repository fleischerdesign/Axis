use crate::app_context::AppContext;
use crate::services::nightlight::NightlightCmd;
use crate::widgets::quick_settings::components::QsTile;
use gtk4::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub struct NightlightPage {
    pub container: gtk4::Box,
}

impl NightlightPage {
    pub fn new(
        ctx: AppContext,
        back_callback: impl Fn() + 'static,
        nl_tile: Rc<QsTile>,
        nightlight_tx: async_channel::Sender<NightlightCmd>,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        container.set_margin_top(16);

        // --- HEADER ---
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["qs-back-btn".to_string()])
            .build();

        let title = gtk4::Label::builder()
            .label("Night Light")
            .halign(gtk4::Align::Start)
            .css_classes(vec!["qs-subpage-title".to_string()])
            .build();

        header.append(&back_btn);
        header.append(&title);

        // --- TOGGLE ROW ---
        let toggle_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        toggle_row.set_hexpand(true);

        let toggle_label = gtk4::Label::builder()
            .label("Enable")
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();

        let toggle = gtk4::Switch::new();

        toggle_row.append(&toggle_label);
        toggle_row.append(&toggle);

        // --- TEMPERATURE SLIDER ---
        let temp_label_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        temp_label_row.set_hexpand(true);

        let temp_label = gtk4::Label::builder()
            .label("Temperature")
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();

        let temp_value = gtk4::Label::builder()
            .label("4500 K")
            .halign(gtk4::Align::End)
            .build();

        temp_label_row.append(&temp_label);
        temp_label_row.append(&temp_value);

        let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 1000.0, 10000.0, 100.0);
        slider.set_hexpand(true);
        slider.add_mark(4500.0, gtk4::PositionType::Bottom, Some("4500 K"));
        slider.add_css_class("volume-slider");

        // --- LAYOUT ---
        container.append(&header);
        container.append(&toggle_row);
        container.append(&temp_label_row);
        container.append(&slider);

        // --- INITIAL STATE ---
        let initial = ctx.nightlight.get();
        toggle.set_state(initial.enabled);
        slider.set_value(initial.temperature as f64);
        temp_value.set_text(&format!("{} K", initial.temperature));

        // --- BINDINGS ---

        // Flag: true = wir setzen den Slider gerade vom Service aus → Handler ignorieren
        let updating_from_service: Rc<Cell<bool>> = Rc::new(Cell::new(false));

        // Slider → Service
        let slider_tx = nightlight_tx.clone();
        let temp_value_label = temp_value.clone();
        let updating_ref = updating_from_service.clone();
        slider.connect_value_changed(move |s| {
            // Ignorieren wenn wir gerade programmatisch updaten
            if updating_ref.get() {
                return;
            }
            let val = s.value() as u32;
            temp_value_label.set_text(&format!("{} K", val));
            let _ = slider_tx.send_blocking(NightlightCmd::SetTemperature(val));
        });

        // Toggle → Service
        let toggle_tx = nightlight_tx.clone();
        toggle.connect_state_notify(move |sw| {
            let _ = toggle_tx.send_blocking(NightlightCmd::Toggle(sw.state()));
        });

        // Service → UI (Toggle + Tile)
        let toggle_update = toggle.clone();
        let nl_tile_service = nl_tile.clone();
        ctx.nightlight.subscribe(move |data| {
            toggle_update.set_state(data.enabled);
            nl_tile_service.set_active(data.enabled);
        });

        // Service → UI (Slider + Label) — mit Block-Flag
        let slider_update = slider.clone();
        let temp_value_update = temp_value.clone();
        let updating_set = updating_from_service.clone();
        ctx.nightlight.subscribe(move |data| {
            let current = slider_update.value() as u32;
            if current != data.temperature {
                updating_set.set(true);
                slider_update.set_value(data.temperature as f64);
                updating_set.set(false);
            }
            temp_value_update.set_text(&format!("{} K", data.temperature));
        });

        // Tile bei Toggle-Change updaten
        let nl_tile_toggle = nl_tile.clone();
        toggle.connect_state_notify(move |sw| {
            nl_tile_toggle.set_active(sw.state());
        });

        back_btn.connect_clicked(move |_| back_callback());

        Self { container }
    }
}

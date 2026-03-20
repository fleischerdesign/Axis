use crate::app_context::AppContext;
use crate::services::nightlight::NightlightCmd;
use crate::widgets::quick_settings::components::header::QsSubPageHeader;
use crate::widgets::QsTile;
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

        let header = QsSubPageHeader::new("Night Light");
        container.append(&header.container);

        // back_btn wiring moved to end of function

        // --- CONTENT BOX (The "Box" like in BT/WiFi) ---
        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list".to_string()])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let content_wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content_wrapper.add_css_class("qs-scrolled");
        content_wrapper.append(&list);
        container.append(&content_wrapper);

        // --- TOGGLE ROW ---
        let toggle_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        toggle_row.set_hexpand(true);
        toggle_row.set_margin_start(12);
        toggle_row.set_margin_end(12);
        toggle_row.set_margin_top(8);
        toggle_row.set_margin_bottom(8);

        let toggle_label = gtk4::Label::builder()
            .label("Enable")
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();

        let toggle = gtk4::Switch::new();

        toggle_row.append(&toggle_label);
        toggle_row.append(&toggle);
        list.append(&toggle_row);

        // --- TEMPERATURES ---
        let create_temp_section = |label_text: &str, initial_val: u32| {
            let section = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            section.set_margin_start(12);
            section.set_margin_end(12);
            section.set_margin_top(8);
            section.set_margin_bottom(8);

            let label_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            let label = gtk4::Label::builder()
                .label(label_text)
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build();
            let val_label = gtk4::Label::new(Some(&format!("{} K", initial_val)));
            label_row.append(&label);
            label_row.append(&val_label);

            let slider =
                gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 1000.0, 10000.0, 100.0);
            slider.set_hexpand(true);
            slider.set_value(initial_val as f64);
            slider.add_css_class("volume-slider");

            section.append(&label_row);
            section.append(&slider);
            (section, slider, val_label)
        };

        let initial = ctx.nightlight.get();

        let (day_sec, day_slider, day_val) =
            create_temp_section("Day Temperature", initial.temp_day);
        let (night_sec, night_slider, night_val) =
            create_temp_section("Night Temperature", initial.temp_night);

        list.append(&day_sec);
        list.append(&night_sec);

        // --- SCHEDULE ---
        let schedule_section = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        schedule_section.set_margin_start(12);
        schedule_section.set_margin_end(12);
        schedule_section.set_margin_top(12);
        schedule_section.set_margin_bottom(12);

        schedule_section.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
        schedule_section.append(
            &gtk4::Label::builder()
                .label("Manual Schedule")
                .halign(gtk4::Align::Start)
                .css_classes(vec!["qs-tile-label".to_string()])
                .build(),
        );

        let schedule_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let sunrise_entry = gtk4::Entry::builder()
            .placeholder_text("Sunrise (07:00)")
            .hexpand(true)
            .build();
        let sunset_entry = gtk4::Entry::builder()
            .placeholder_text("Sunset (20:00)")
            .hexpand(true)
            .build();
        sunrise_entry.set_text(&initial.sunrise);
        sunset_entry.set_text(&initial.sunset);

        schedule_row.append(&sunrise_entry);
        schedule_row.append(&sunset_entry);
        schedule_section.append(&schedule_row);
        list.append(&schedule_section);

        // --- BINDINGS ---
        let updating_from_service = Rc::new(Cell::new(false));

        // Day Slider -> Service
        let tx = nightlight_tx.clone();
        let val_lbl = day_val.clone();
        let updating = updating_from_service.clone();
        day_slider.connect_value_changed(move |s| {
            if updating.get() {
                return;
            }
            let val = s.value() as u32;
            val_lbl.set_text(&format!("{} K", val));
            let _ = tx.send_blocking(NightlightCmd::SetTempDay(val));
        });

        // Night Slider -> Service
        let tx = nightlight_tx.clone();
        let val_lbl = night_val.clone();
        let updating = updating_from_service.clone();
        night_slider.connect_value_changed(move |s| {
            if updating.get() {
                return;
            }
            let val = s.value() as u32;
            val_lbl.set_text(&format!("{} K", val));
            let _ = tx.send_blocking(NightlightCmd::SetTempNight(val));
        });

        // Toggle -> Service
        let tx = nightlight_tx.clone();
        toggle.connect_state_notify(move |sw| {
            let _ = tx.send_blocking(NightlightCmd::Toggle(sw.state()));
        });

        // Schedule: Apply on Enter
        let apply_schedule = {
            let tx = nightlight_tx.clone();
            let sun_e = sunrise_entry.clone();
            let set_e = sunset_entry.clone();
            move || {
                let _ = tx.send_blocking(NightlightCmd::SetSchedule(
                    sun_e.text().to_string(),
                    set_e.text().to_string(),
                ));
            }
        };

        let apply_c1 = apply_schedule.clone();
        sunrise_entry.connect_activate(move |_| apply_c1());
        let apply_c2 = apply_schedule.clone();
        sunset_entry.connect_activate(move |_| apply_c2());

        // Service -> UI
        let toggle_c = toggle.clone();
        let nl_tile_c = nl_tile.clone();
        let day_slider_c = day_slider.clone();
        let night_slider_c = night_slider.clone();
        let day_val_c = day_val.clone();
        let night_val_c = night_val.clone();
        let sun_e_c = sunrise_entry.clone();
        let set_e_c = sunset_entry.clone();
        let updating_c = updating_from_service.clone();

        ctx.nightlight.subscribe(move |data| {
            updating_c.set(true);
            toggle_c.set_state(data.enabled);
            nl_tile_c.set_active(data.enabled);

            day_slider_c.set_value(data.temp_day as f64);
            day_val_c.set_text(&format!("{} K", data.temp_day));

            night_slider_c.set_value(data.temp_night as f64);
            night_val_c.set_text(&format!("{} K", data.temp_night));

            // Nur updaten, wenn der User gerade nicht tippt (Fokus-Check wäre besser, aber Textvergleich reicht meist)
            if sun_e_c.text() != data.sunrise {
                sun_e_c.set_text(&data.sunrise);
            }
            if set_e_c.text() != data.sunset {
                set_e_c.set_text(&data.sunset);
            }

            updating_c.set(false);
        });

        header.connect_back(back_callback);

        Self { container }
    }
}

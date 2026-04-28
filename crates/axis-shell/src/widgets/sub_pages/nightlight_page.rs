use gtk4::prelude::*;
use axis_domain::models::nightlight::NightlightStatus;
use axis_presentation::View;
use crate::presentation::nightlight::NightlightPresenter;
use crate::widgets::components::popup_header::PopupHeader;
use std::cell::Cell;
use std::rc::Rc;

pub struct NightlightPage {
    pub container: gtk4::Box,
}

impl NightlightPage {
    pub fn new(presenter: Rc<NightlightPresenter>, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);

        let header = PopupHeader::new("Night Light");
        header.connect_back(on_back);
        container.append(&header.container);

        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list".to_string()])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        container.append(&list);

        let toggle_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
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

        let (day_sec, day_slider, day_val) = Self::create_temp_section("Day Temperature", 6500);
        let (night_sec, night_slider, night_val) = Self::create_temp_section("Night Temperature", 4500);
        list.append(&day_sec);
        list.append(&night_sec);

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
                .css_classes(vec!["tile-label".to_string()])
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
        schedule_row.append(&sunrise_entry);
        schedule_row.append(&sunset_entry);
        schedule_section.append(&schedule_row);
        list.append(&schedule_section);

        let updating_from_service = Rc::new(Cell::new(false));

        let pres_toggle = presenter.clone();
        let upd_toggle = updating_from_service.clone();
        toggle.connect_state_notify(move |sw| {
            if upd_toggle.get() { return; }
            pres_toggle.set_enabled(sw.state());
        });

        let pres_day = presenter.clone();
        let day_val_c = day_val.clone();
        let upd_day = updating_from_service.clone();
        day_slider.connect_value_changed(move |s| {
            if upd_day.get() { return; }
            let val = s.value() as u32;
            day_val_c.set_text(&format!("{} K", val));
            pres_day.set_temp_day(val);
        });

        let pres_night = presenter.clone();
        let night_val_c = night_val.clone();
        let upd_night = updating_from_service.clone();
        night_slider.connect_value_changed(move |s| {
            if upd_night.get() { return; }
            let val = s.value() as u32;
            night_val_c.set_text(&format!("{} K", val));
            pres_night.set_temp_night(val);
        });

        let pres_sched = presenter.clone();
        let apply_schedule = {
            let sunrise_e = sunrise_entry.clone();
            let sunset_e = sunset_entry.clone();
            move || {
                pres_sched.set_schedule(sunrise_e.text().to_string(), sunset_e.text().to_string());
            }
        };

        let apply_c1 = apply_schedule.clone();
        sunrise_entry.connect_activate(move |_| apply_c1());
        let apply_c2 = apply_schedule;
        sunset_entry.connect_activate(move |_| apply_c2());

        let view = Box::new(NightlightPageView {
            toggle: toggle.clone(),
            day_slider: day_slider.clone(),
            night_slider: night_slider.clone(),
            day_val: day_val.clone(),
            night_val: night_val.clone(),
            sunrise_entry: sunrise_entry.clone(),
            sunset_entry: sunset_entry.clone(),
            updating: updating_from_service.clone(),
        });
        presenter.add_view(view);

        Self { container }
    }

    fn create_temp_section(label_text: &str, initial_val: u32) -> (gtk4::Box, gtk4::Scale, gtk4::Label) {
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

        let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 1000.0, 10000.0, 100.0);
        slider.set_hexpand(true);
        slider.set_value(initial_val as f64);
        slider.set_draw_value(false);
        slider.add_css_class("volume-slider");

        section.append(&label_row);
        section.append(&slider);
        (section, slider, val_label)
    }
}

struct NightlightPageView {
    toggle: gtk4::Switch,
    day_slider: gtk4::Scale,
    night_slider: gtk4::Scale,
    day_val: gtk4::Label,
    night_val: gtk4::Label,
    sunrise_entry: gtk4::Entry,
    sunset_entry: gtk4::Entry,
    updating: Rc<Cell<bool>>,
}

impl View<NightlightStatus> for NightlightPageView {
    fn render(&self, status: &NightlightStatus) {
        self.updating.set(true);

        self.toggle.set_state(status.enabled);

        self.day_slider.set_value(status.temp_day as f64);
        self.day_val.set_text(&format!("{} K", status.temp_day));

        self.night_slider.set_value(status.temp_night as f64);
        self.night_val.set_text(&format!("{} K", status.temp_night));

        if self.sunrise_entry.text() != status.sunrise {
            self.sunrise_entry.set_text(&status.sunrise);
        }
        if self.sunset_entry.text() != status.sunset {
            self.sunset_entry.set_text(&status.sunset);
        }

        self.updating.set(false);
    }
}


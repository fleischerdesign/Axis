use gtk4::prelude::*;
use libadwaita::prelude::*;
use axis_domain::models::audio::{AudioStatus, AudioDevice};
use crate::presentation::audio::{AudioPresenter, AudioView, audio_icon};
use crate::presentation::presenter::View;
use crate::widgets::components::popup_header::PopupHeader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct AudioPage {
    pub container: gtk4::Box,
}

impl AudioPage {
    pub fn new(presenter: Rc<AudioPresenter>, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);

        let header = PopupHeader::new("Audio");
        header.connect_back(on_back);
        container.append(&header.container);

        let sink_model = gtk4::StringList::new(&[]);
        let sink_combo = libadwaita::ComboRow::builder()
            .title("Output Device")
            .model(&sink_model)
            .use_subtitle(true)
            .build();
        let sink_group = libadwaita::PreferencesGroup::new();
        sink_group.add(&sink_combo);
        container.append(&sink_group);

        let source_model = gtk4::StringList::new(&[]);
        let source_combo = libadwaita::ComboRow::builder()
            .title("Input Device")
            .model(&source_model)
            .use_subtitle(true)
            .build();
        let source_group = libadwaita::PreferencesGroup::new();
        source_group.add(&source_combo);
        container.append(&source_group);

        let master_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        master_row.add_css_class("audio-master-row");

        let master_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        master_icon.set_pixel_size(18);

        let master_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        master_slider.set_hexpand(true);
        master_slider.set_draw_value(false);
        master_slider.add_css_class("volume-slider");

        master_row.append(&master_icon);
        master_row.append(&master_slider);
        container.append(&master_row);

        let app_list = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .min_content_height(150)
            .build();
        scrolled.set_child(Some(&app_list));
        container.append(&scrolled);

        let empty_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        empty_box.set_valign(gtk4::Align::Center);
        empty_box.set_vexpand(true);
        empty_box.set_visible(false);

        let empty_icon = gtk4::Image::from_icon_name("audio-volume-muted-symbolic");
        empty_icon.set_pixel_size(32);
        empty_icon.add_css_class("audio-empty-icon");

        let empty_label = gtk4::Label::new(Some("No applications playing audio"));
        empty_label.add_css_class("audio-empty-label");

        empty_box.append(&empty_icon);
        empty_box.append(&empty_label);
        container.append(&empty_box);

        let sink_ids: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
        let source_ids: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
        let is_rebuilding = Rc::new(Cell::new(false));
        let sink_user_selecting = Rc::new(Cell::new(false));
        let source_user_selecting = Rc::new(Cell::new(false));

        let master_icon_c = master_icon.clone();
        let sink_combo_c = sink_combo.clone();
        let sink_model_c = sink_model.clone();
        let source_combo_c = source_combo.clone();
        let source_model_c = source_model.clone();
        let sink_ids_c = sink_ids.clone();
        let source_ids_c = source_ids.clone();
        let sink_user_sub = sink_user_selecting.clone();
        let source_user_sub = source_user_selecting.clone();
        let app_list_c = app_list.clone();
        let empty_box_c = empty_box.clone();
        let is_rebuilding_c = is_rebuilding.clone();
        let _pres_render = presenter.clone();

        let view = Box::new(AudioPageView {
            master_icon: master_icon_c,
            sink_combo: sink_combo_c,
            sink_model: sink_model_c,
            source_combo: source_combo_c,
            source_model: source_model_c,
            sink_ids: sink_ids_c,
            source_ids: source_ids_c,
            sink_user_selecting: sink_user_sub,
            source_user_selecting: source_user_sub,
            app_list: app_list_c,
            empty_box: empty_box_c,
            is_rebuilding: is_rebuilding_c,
        });
        presenter.add_view(view);

        let pres_sink = presenter.clone();
        let sink_user_c = sink_user_selecting.clone();
        let sink_ids_sel = sink_ids.clone();
        sink_combo.connect_selected_notify(move |combo| {
            if !sink_user_c.get() { return; }
            let idx = combo.selected() as usize;
            if let Some(&id) = sink_ids_sel.borrow().get(idx) {
                pres_sink.set_default_sink(id);
            }
        });

        let pres_source = presenter.clone();
        let source_user_c = source_user_selecting.clone();
        let source_ids_sel = source_ids.clone();
        source_combo.connect_selected_notify(move |combo| {
            if !source_user_c.get() { return; }
            let idx = combo.selected() as usize;
            if let Some(&id) = source_ids_sel.borrow().get(idx) {
                pres_source.set_default_source(id);
            }
        });

        let pres_vol = presenter.clone();
        master_slider.connect_value_changed(move |s| {
            pres_vol.handle_user_volume_change(s.value());
        });

        Self { container }
    }
}

struct AudioPageView {
    master_icon: gtk4::Image,
    sink_combo: libadwaita::ComboRow,
    sink_model: gtk4::StringList,
    source_combo: libadwaita::ComboRow,
    source_model: gtk4::StringList,
    sink_ids: Rc<RefCell<Vec<u32>>>,
    source_ids: Rc<RefCell<Vec<u32>>>,
    sink_user_selecting: Rc<Cell<bool>>,
    source_user_selecting: Rc<Cell<bool>>,
    app_list: gtk4::Box,
    empty_box: gtk4::Box,
    is_rebuilding: Rc<Cell<bool>>,
}

impl View<AudioStatus> for AudioPageView {
    fn render(&self, status: &AudioStatus) {
        let icon_name = audio_icon(status).to_string();
        let icon_c = self.master_icon.clone();
        gtk4::glib::idle_add_local(move || {
            icon_c.set_icon_name(Some(&icon_name));
            gtk4::glib::ControlFlow::Break
        });

        self.update_combo(&self.sink_model, &self.sink_combo, &status.sinks, &self.sink_user_selecting);
        self.update_combo(&self.source_model, &self.source_combo, &status.sources, &self.source_user_selecting);

        *self.sink_ids.borrow_mut() = status.sinks.iter().map(|s| s.id).collect();
        *self.source_ids.borrow_mut() = status.sources.iter().map(|s| s.id).collect();

        if status.sink_inputs.is_empty() {
            if self.app_list.first_child().is_some() {
                while let Some(child) = self.app_list.first_child() {
                    self.app_list.remove(&child);
                }
            }
            self.empty_box.set_visible(true);
            return;
        }

        self.empty_box.set_visible(false);

        if self.is_rebuilding.get() { return; }
        self.is_rebuilding.set(true);

        while let Some(child) = self.app_list.first_child() {
            self.app_list.remove(&child);
        }

        for input in &status.sink_inputs {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            row.add_css_class("audio-app-row");
            row.set_margin_top(4);
            row.set_margin_bottom(4);

            let name_label = gtk4::Label::builder()
                .label(&input.name)
                .halign(gtk4::Align::Start)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .css_classes(vec!["audio-app-name".to_string()])
                .build();
            name_label.set_width_request(100);

            let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
            slider.set_hexpand(true);
            slider.set_draw_value(false);
            slider.set_value(input.volume);
            slider.add_css_class("volume-slider");

            row.append(&name_label);
            row.append(&slider);
            self.app_list.append(&row);
        }

        self.is_rebuilding.set(false);
    }
}

impl AudioPageView {
    fn update_combo(
        &self,
        model: &gtk4::StringList,
        combo: &libadwaita::ComboRow,
        items: &[AudioDevice],
        user_selecting: &Rc<Cell<bool>>,
    ) {
        let default_idx = items.iter().position(|d| d.is_default);
        let names: Vec<&str> = items.iter().map(|d| d.description.as_str()).collect();

        let current_count = model.n_items();
        let needs_rebuild = current_count as usize != items.len()
            || items
                .iter()
                .enumerate()
                .any(|(i, d)| model.string(i as u32).as_deref() != Some(d.description.as_str()));

        if needs_rebuild {
            user_selecting.set(false);
            model.splice(0, current_count, &names);
            if let Some(idx) = default_idx {
                combo.set_selected(idx as u32);
            }
            user_selecting.set(true);
        } else if let Some(idx) = default_idx {
            if combo.selected() != idx as u32 {
                user_selecting.set(false);
                combo.set_selected(idx as u32);
                user_selecting.set(true);
            }
        }
    }
}

impl AudioView for AudioPageView {
    fn on_volume_changed(&self, _f: Box<dyn Fn(f64) + 'static>) {}
    fn on_set_default_sink(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_default_source(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_sink_input_volume(&self, _f: Box<dyn Fn(u32, f64) + 'static>) {}
}

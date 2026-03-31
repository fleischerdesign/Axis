use crate::app_context::AppContext;
use axis_core::services::audio::{AudioCmd, AudioData, SinkInputData};
use crate::widgets::components::debounced_slider::DebouncedSlider;
use crate::widgets::components::icon_slider::IconSlider;
use crate::widgets::components::subpage_header::SubPageHeader;
use crate::widgets::icons;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct AudioPage {
    pub container: gtk4::Box,
}

impl AudioPage {
    pub fn new(ctx: AppContext, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        let header = SubPageHeader::new("Audio", None::<&gtk4::Widget>);
        header.connect_back(on_back);
        container.append(&header.container);

        // --- OUTPUT / INPUT DEVICE SELECTORS ---
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

        // --- MASTER SLIDER (reuses DebouncedSlider) ---
        let master_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        master_row.add_css_class("audio-master-row");

        let master_slider = DebouncedSlider::new(
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
                move |slider: &IconSlider, data: &AudioData| {
                    let icon_name = icons::volume_icon(data.volume, data.is_muted);
                    slider.set_icon(icon_name);
                }
            }),
            Some({
                move |slider: &IconSlider, val: f64| {
                    slider.set_icon(icons::volume_icon(val, false));
                }
            }),
        );
        master_slider
            .icon_slider
            .overlay
            .add_css_class("volume-slider");
        master_row.append(&master_slider.icon_slider.overlay);
        container.append(&master_row);

        // --- APP LIST ---
        let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .min_content_height(150)
            .build();
        scrolled.set_child(Some(&list_box));

        container.append(&scrolled);

        // --- EMPTY STATE ---
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

        // --- REACTIVE UPDATES ---
        let list_box_c = list_box.clone();
        let empty_box_c = empty_box.clone();
        let sink_combo_c = sink_combo.clone();
        let sink_model_c = sink_model.clone();
        let source_combo_c = source_combo.clone();
        let source_model_c = source_model.clone();
        let is_rebuilding = Rc::new(Cell::new(false));
        let ctx_row = ctx.clone();

        // Track whether user-initiated selection change should send a command
        let sink_user_selecting = Rc::new(Cell::new(false));
        let source_user_selecting = Rc::new(Cell::new(false));

        // Store device names for index→name mapping
        let sink_names: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let source_names: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        // Clones for subscribe closure
        let sink_user_for_sub = sink_user_selecting.clone();
        let source_user_for_sub = source_user_selecting.clone();
        let sink_names_c = sink_names.clone();
        let source_names_c = source_names.clone();

        ctx.audio.subscribe(move |data| {
            // Update sink list
            update_combo(
                &sink_model_c,
                &sink_combo_c,
                &data.sinks,
                &sink_user_for_sub,
            );

            // Update source list
            update_combo(
                &source_model_c,
                &source_combo_c,
                &data.sources,
                &source_user_for_sub,
            );

            // Store device names for index→name mapping
            *sink_names_c.borrow_mut() = data.sinks.iter().map(|s| s.name.clone()).collect();
            *source_names_c.borrow_mut() = data.sources.iter().map(|s| s.name.clone()).collect();

            // Update app list
            if data.sink_inputs.is_empty() {
                if list_box_c.first_child().is_some() {
                    while let Some(child) = list_box_c.first_child() {
                        list_box_c.remove(&child);
                    }
                }
                empty_box_c.set_visible(true);
                return;
            }

            empty_box_c.set_visible(false);

            if is_rebuilding.get() {
                return;
            }
            is_rebuilding.set(true);

            while let Some(child) = list_box_c.first_child() {
                list_box_c.remove(&child);
            }

            for input in &data.sink_inputs {
                let row = Self::build_app_row(input, &ctx_row);
                list_box_c.append(&row);
            }

            is_rebuilding.set(false);
        });

        // Sink selection → SetDefaultSink
        let tx_sink = ctx.audio.tx.clone();
        let sink_user_c = sink_user_selecting.clone();
        let sink_names_for_sel = sink_names.clone();
        sink_combo.connect_selected_notify(move |combo| {
            if !sink_user_c.get() {
                return;
            }
            let idx = combo.selected() as usize;
            if let Some(name) = sink_names_for_sel.borrow().get(idx) {
                let _ = tx_sink.try_send(AudioCmd::SetDefaultSink(name.clone()));
            }
        });

        // Source selection → SetDefaultSource
        let tx_source = ctx.audio.tx.clone();
        let source_user_c = source_user_selecting.clone();
        let source_names_for_sel = source_names.clone();
        source_combo.connect_selected_notify(move |combo| {
            if !source_user_c.get() {
                return;
            }
            let idx = combo.selected() as usize;
            if let Some(name) = source_names_for_sel.borrow().get(idx) {
                let _ = tx_source.try_send(AudioCmd::SetDefaultSource(name.clone()));
            }
        });

        // Insert empty state below scrolled window
        container.append(&empty_box);

        Self { container }
    }

    fn build_app_row(input: &SinkInputData, ctx: &AppContext) -> gtk4::Box {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        row.add_css_class("audio-app-row");
        row.set_margin_top(4);
        row.set_margin_bottom(4);

        let slider_overlay = gtk4::Overlay::new();
        slider_overlay.add_css_class("volume-slider");
        slider_overlay.set_hexpand(true);

        let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        slider.set_hexpand(true);
        slider.set_value(input.volume);

        let overlay_content = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        overlay_content.set_halign(gtk4::Align::Start);
        overlay_content.set_valign(gtk4::Align::Center);
        overlay_content.set_margin_start(22);
        overlay_content.set_can_target(false);

        let app_icon = gtk4::Image::from_icon_name(&input.icon_name);
        app_icon.set_pixel_size(18);

        let name_label = gtk4::Label::builder()
            .label(&input.name)
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::Center)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(vec!["audio-app-name".to_string()])
            .build();

        overlay_content.append(&app_icon);
        overlay_content.append(&name_label);

        slider_overlay.set_child(Some(&slider));
        slider_overlay.add_overlay(&overlay_content);

        row.append(&slider_overlay);

        let tx = ctx.audio.tx.clone();
        let app_index = input.index;
        slider.connect_value_changed(move |s| {
            let _ = tx.try_send(AudioCmd::SetSinkInputVolume(app_index, s.value()));
        });

        row
    }
}

fn update_combo<T>(
    model: &gtk4::StringList,
    combo: &libadwaita::ComboRow,
    items: &[T],
    user_selecting: &Cell<bool>,
) where
    T: HasComboEntry,
{
    // Find current default index
    let default_idx = items.iter().position(|d| d.is_default());

    // Build display names
    let names: Vec<&str> = items.iter().map(|d| d.description()).collect();

    // Rebuild model if count changed or names differ
    let current_count = model.n_items();
    let needs_rebuild = current_count as usize != items.len()
        || items
            .iter()
            .enumerate()
            .any(|(i, d)| model.string(i as u32).as_deref() != Some(d.description()));

    if needs_rebuild {
        user_selecting.set(false);
        model.splice(0, current_count, &names);
        if let Some(idx) = default_idx {
            combo.set_selected(idx as u32);
        }
        user_selecting.set(true);
    } else if let Some(idx) = default_idx {
        // Just update selection without rebuilding
        if combo.selected() != idx as u32 {
            user_selecting.set(false);
            combo.set_selected(idx as u32);
            user_selecting.set(true);
        }
    }
}

trait HasComboEntry {
    fn description(&self) -> &str;
    fn is_default(&self) -> bool;
}

impl HasComboEntry for axis_core::services::audio::SinkData {
    fn description(&self) -> &str {
        &self.description
    }
    fn is_default(&self) -> bool {
        self.is_default
    }
}

impl HasComboEntry for axis_core::services::audio::SourceData {
    fn description(&self) -> &str {
        &self.description
    }
    fn is_default(&self) -> bool {
        self.is_default
    }
}

use crate::app_context::AppContext;
use crate::services::audio::{AudioCmd, SinkInputData};
use crate::widgets::components::debounced_slider::DebouncedSlider;
use crate::widgets::components::icon_slider::IconSlider;
use crate::widgets::components::subpage_header::SubPageHeader;
use crate::widgets::icons;
use gtk4::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub struct AudioPage {
    pub container: gtk4::Box,
}

impl AudioPage {
    pub fn new(ctx: AppContext, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        let header = SubPageHeader::new("Audio");
        header.connect_back(on_back);
        container.append(&header.container);

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
                move |slider: &IconSlider, data: &crate::services::audio::AudioData| {
                    let icon_name = icons::volume_icon(data.volume, data.is_muted);
                    slider.set_icon(icon_name);
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

        // Reactive: rebuild app list on audio changes
        let list_box_c = list_box.clone();
        let is_rebuilding = Rc::new(Cell::new(false));
        let ctx_row = ctx.clone();
        ctx.audio.subscribe(move |data| {
            if data.sink_inputs.is_empty() {
                if list_box_c.first_child().is_some() {
                    // No sink inputs — clear stale rows
                    while let Some(child) = list_box_c.first_child() {
                        list_box_c.remove(&child);
                    }
                }
                return;
            }
            if is_rebuilding.get() {
                return;
            }
            is_rebuilding.set(true);

            // Clear existing rows
            while let Some(child) = list_box_c.first_child() {
                list_box_c.remove(&child);
            }

            for input in &data.sink_inputs {
                let row = Self::build_app_row(input, &ctx_row);
                list_box_c.append(&row);
            }

            is_rebuilding.set(false);
        });

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

        // Slider → SetSinkInputVolume
        let tx = ctx.audio.tx.clone();
        let app_index = input.index;
        slider.connect_value_changed(move |s| {
            let _ = tx.try_send(AudioCmd::SetSinkInputVolume(app_index, s.value()));
        });

        row
    }
}

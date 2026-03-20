use crate::app_context::AppContext;
use crate::services::audio::{AudioCmd, SinkInputData};
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

        // --- HEADER ---
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["qs-back-btn".to_string()])
            .build();

        let title = gtk4::Label::builder()
            .label("Audio")
            .halign(gtk4::Align::Start)
            .css_classes(vec!["qs-subpage-title".to_string()])
            .build();

        header.append(&back_btn);
        header.append(&title);
        container.append(&header);

        back_btn.connect_clicked(move |_| {
            on_back();
        });

        // --- MASTER SLIDER (pill style like main page) ---
        let master_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        master_row.add_css_class("audio-master-row");

        let master_overlay = gtk4::Overlay::new();
        master_overlay.add_css_class("volume-slider");
        master_overlay.set_hexpand(true);

        let master_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        master_slider.set_hexpand(true);

        let master_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        master_icon.set_pixel_size(22);
        master_icon.set_margin_start(22);
        master_icon.set_halign(gtk4::Align::Start);
        master_icon.set_valign(gtk4::Align::Center);
        master_icon.set_can_target(false);

        master_overlay.set_child(Some(&master_slider));
        master_overlay.add_overlay(&master_icon);

        master_row.append(&master_overlay);
        container.append(&master_row);

        // Master slider reactive bindings (debounce like main page)
        let is_updating = Rc::new(std::cell::RefCell::new(false));

        let master_slider_c = master_slider.clone();
        let master_icon_c = master_icon.clone();
        let is_updating_rx = is_updating.clone();
        ctx.audio.subscribe(move |data| {
            if !*is_updating_rx.borrow() {
                *is_updating_rx.borrow_mut() = true;
                master_slider_c.set_value(data.volume);
                *is_updating_rx.borrow_mut() = false;
            }
            let icon_name = icons::volume_icon(data.volume, data.is_muted);
            master_icon_c.set_icon_name(Some(icon_name));
        });

        let ctx_audio = ctx.clone();
        let is_updating_cmd = is_updating.clone();
        master_slider.connect_value_changed(move |s| {
            if *is_updating_cmd.borrow() {
                return;
            }
            let _ = ctx_audio
                .audio_tx
                .send_blocking(AudioCmd::SetVolume(s.value()));
        });

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
        let tx = ctx.audio_tx.clone();
        let app_index = input.index;
        slider.connect_value_changed(move |s| {
            let _ = tx.send_blocking(AudioCmd::SetSinkInputVolume(app_index, s.value()));
        });

        row
    }
}

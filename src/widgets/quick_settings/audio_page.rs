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

        // --- MASTER SLIDER ---
        let master_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        master_row.add_css_class("audio-master-row");

        let master_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        master_icon.set_pixel_size(18);
        master_icon.set_margin_start(4);
        master_icon.set_valign(gtk4::Align::Center);

        let master_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        master_slider.set_hexpand(true);
        master_slider.set_margin_start(8);
        master_slider.set_margin_end(8);

        let master_pct = gtk4::Label::builder()
            .label("100%")
            .css_classes(vec!["audio-pct-label".to_string()])
            .width_chars(4)
            .xalign(1.0)
            .build();

        let master_mute = gtk4::Button::builder()
            .icon_name("audio-volume-high-symbolic")
            .css_classes(vec!["audio-mute-btn".to_string()])
            .tooltip_text("Mute")
            .build();

        master_row.append(&master_icon);
        master_row.append(&master_slider);
        master_row.append(&master_pct);
        master_row.append(&master_mute);
        container.append(&master_row);

        // Master slider reactive bindings (debounce like main page)
        let is_updating = Rc::new(std::cell::RefCell::new(false));

        let master_slider_c = master_slider.clone();
        let master_icon_c = master_icon.clone();
        let master_pct_c = master_pct.clone();
        let is_updating_rx = is_updating.clone();
        ctx.audio.subscribe(move |data| {
            if !*is_updating_rx.borrow() {
                *is_updating_rx.borrow_mut() = true;
                master_slider_c.set_value(data.volume);
                *is_updating_rx.borrow_mut() = false;
            }
            let icon_name = icons::volume_icon(data.volume, data.is_muted);
            master_icon_c.set_icon_name(Some(icon_name));
            master_pct_c.set_text(&format!("{:.0}%", (data.volume * 100.0).round()));
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

        // Mute toggle
        let ctx_audio_m = ctx.clone();
        let audio_store_m = ctx.audio.clone();
        master_mute.connect_clicked(move |btn| {
            let data = audio_store_m.get();
            let new_mute = !data.is_muted;
            btn.set_icon_name(if new_mute {
                "audio-volume-muted-symbolic"
            } else {
                "audio-volume-high-symbolic"
            });
            let _ = ctx_audio_m
                .audio_tx
                .send_blocking(AudioCmd::SetMute(new_mute));
        });

        // --- SEPARATOR ---
        let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        sep.set_margin_top(4);
        sep.set_margin_bottom(4);
        container.append(&sep);

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
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        row.add_css_class("audio-app-row");
        row.set_margin_top(4);
        row.set_margin_bottom(4);

        let icon = gtk4::Image::from_icon_name(&input.icon_name);
        icon.set_pixel_size(18);
        icon.set_valign(gtk4::Align::Center);

        let name_label = gtk4::Label::builder()
            .label(&input.name)
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::Center)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(vec!["audio-app-name".to_string()])
            .build();

        let slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        slider.set_hexpand(true);
        slider.set_value(input.volume);

        let mute_btn = gtk4::Button::builder()
            .icon_name(if input.is_muted {
                "audio-volume-muted-symbolic"
            } else {
                icons::volume_icon(input.volume, input.is_muted)
            })
            .css_classes(vec!["audio-app-mute".to_string()])
            .tooltip_text("Mute")
            .build();

        row.append(&icon);
        row.append(&name_label);
        row.append(&slider);
        row.append(&mute_btn);

        // Slider → SetSinkInputVolume
        let tx = ctx.audio_tx.clone();
        let app_index = input.index;
        slider.connect_value_changed(move |s| {
            let _ = tx.send_blocking(AudioCmd::SetSinkInputVolume(app_index, s.value()));
        });

        // Mute toggle
        let tx_m = ctx.audio_tx.clone();
        let app_index_m = input.index;
        let slider_c = slider.clone();
        let mute_btn_c = mute_btn.clone();
        let was_muted = Rc::new(Cell::new(input.is_muted));
        mute_btn.connect_clicked(move |_| {
            let new_mute = !was_muted.get();
            was_muted.set(new_mute);
            mute_btn_c.set_icon_name(if new_mute {
                "audio-volume-muted-symbolic"
            } else {
                icons::volume_icon(slider_c.value(), false)
            });
            let _ = tx_m.send_blocking(AudioCmd::SetSinkInputMute(app_index_m, new_mute));
        });

        row
    }
}

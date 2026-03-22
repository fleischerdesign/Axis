use crate::app_context::AppContext;
use crate::services::kdeconnect::{KdeConnectCmd, KdeConnectDeviceData};
use crate::widgets::components::scrolled_list::ScrolledList;
use crate::widgets::components::subpage_header::SubPageHeader;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct KdeConnectPage {
    pub container: gtk4::Box,
}

fn device_icon(device_type: &str) -> &str {
    match device_type {
        "phone" => "phone-symbolic",
        "tablet" => "computer-apple-ipad-symbolic",
        "desktop" => "computer-symbolic",
        "laptop" => "laptop-symbolic",
        _ => "phone-symbolic",
    }
}

fn build_device_row(
    device: &KdeConnectDeviceData,
    tx: &async_channel::Sender<KdeConnectCmd>,
) -> gtk4::Box {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    row.set_margin_start(12);
    row.set_margin_end(12);
    row.set_margin_top(8);
    row.set_margin_bottom(8);

    // Device icon (left side)
    let dev_icon = gtk4::Image::from_icon_name(device_icon(&device.device_type));
    dev_icon.set_pixel_size(24);
    dev_icon.set_valign(gtk4::Align::Center);
    row.append(&dev_icon);

    // Device info (name + status stacked)
    let info = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    info.set_hexpand(true);
    info.set_valign(gtk4::Align::Center);

    let name_label = gtk4::Label::builder()
        .label(&device.name)
        .halign(gtk4::Align::Start)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .build();
    info.append(&name_label);

    // Status sublabel
    let mut status_parts = Vec::new();
    if device.is_paired && device.is_reachable {
        status_parts.push("Verbunden".to_string());
    } else if device.is_paired {
        status_parts.push("Gekoppelt".to_string());
    } else if device.is_reachable {
        status_parts.push("Verfügbar".to_string());
    }
    if let Some(level) = device.battery_level {
        let charging_str = if device.battery_charging { " ⚡" } else { "" };
        status_parts.push(format!("{level}%{charging_str}"));
    }

    if !status_parts.is_empty() {
        let status_label = gtk4::Label::builder()
            .label(&status_parts.join(" · "))
            .halign(gtk4::Align::Start)
            .css_classes(vec!["list-sublabel".to_string()])
            .build();
        info.append(&status_label);
    }

    row.append(&info);

    // Action buttons
    let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);

    if device.is_paired && device.is_reachable {
        if device.has_ping {
            let ping_btn = gtk4::Button::from_icon_name("dialog-information-symbolic");
            ping_btn.set_tooltip_text(Some("Ping"));
            ping_btn.add_css_class("flat");
            ping_btn.add_css_class("circular");
            let tx_c = tx.clone();
            let id = device.id.clone();
            ping_btn.connect_clicked(move |_| {
                let _ = tx_c.try_send(KdeConnectCmd::Ping {
                    device_id: id.clone(),
                });
            });
            actions.append(&ping_btn);
        }

        if device.has_findmyphone {
            let ring_btn = gtk4::Button::from_icon_name("call-start-symbolic");
            ring_btn.set_tooltip_text(Some("Klingeln"));
            ring_btn.add_css_class("flat");
            ring_btn.add_css_class("circular");
            let tx_c = tx.clone();
            let id = device.id.clone();
            ring_btn.connect_clicked(move |_| {
                let _ = tx_c.try_send(KdeConnectCmd::Ring {
                    device_id: id.clone(),
                });
            });
            actions.append(&ring_btn);
        }
    }

    if device.is_reachable {
        let pair_btn = if device.is_paired {
            let btn = gtk4::Button::from_icon_name("network-transmit-receive-symbolic");
            btn.set_tooltip_text(Some("Trennen"));
            btn.add_css_class("flat");
            btn.add_css_class("circular");
            let tx_c = tx.clone();
            let id = device.id.clone();
            btn.connect_clicked(move |_| {
                let _ = tx_c.try_send(KdeConnectCmd::Unpair {
                    device_id: id.clone(),
                });
            });
            btn
        } else {
            let btn = gtk4::Button::from_icon_name("network-transmit-receive-symbolic");
            btn.set_tooltip_text(Some("Koppeln"));
            btn.add_css_class("flat");
            btn.add_css_class("circular");
            let tx_c = tx.clone();
            let id = device.id.clone();
            btn.connect_clicked(move |_| {
                let _ = tx_c.try_send(KdeConnectCmd::Pair {
                    device_id: id.clone(),
                });
            });
            btn
        };
        actions.append(&pair_btn);
    }

    row.append(&actions);
    row
}

impl KdeConnectPage {
    pub fn new(ctx: AppContext, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        let header = SubPageHeader::new("KDE Connect");
        container.append(&header.container);

        let scrolled_list = ScrolledList::new(300);
        scrolled_list.list.add_css_class("qs-list");
        container.append(&scrolled_list.scrolled);

        let on_back = Rc::new(on_back);
        header.connect_back(move || {
            on_back();
        });

        let list_c = scrolled_list.list;
        let tx_row = ctx.kdeconnect.tx.clone();

        ctx.kdeconnect.subscribe(move |data| {
            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }

            if !data.available {
                let label = gtk4::Label::builder()
                    .label("kdeconnectd nicht verfügbar")
                    .css_classes(vec!["list-sublabel".to_string()])
                    .margin_top(20)
                    .build();
                list_c.append(&label);
                return;
            }

            for device in &data.devices {
                let device_box = build_device_row(device, &tx_row);
                let frame = gtk4::Frame::new(None);
                frame.add_css_class("list-row");
                frame.set_child(Some(&device_box));
                list_c.append(&frame);
            }

            if data.devices.is_empty() {
                let label = gtk4::Label::builder()
                    .label("Keine Geräte gefunden")
                    .css_classes(vec!["list-sublabel".to_string()])
                    .margin_top(20)
                    .build();
                list_c.append(&label);
            }
        });

        Self { container }
    }
}

use crate::app_context::AppContext;
use crate::services::continuity::{ContinuityCmd, SharingMode};
use crate::widgets::components::scrolled_list::ScrolledList;
use crate::widgets::components::subpage_header::SubPageHeader;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct ContinuityPage {
    pub container: gtk4::Box,
}

impl ContinuityPage {
    pub fn new(ctx: AppContext, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        let header = SubPageHeader::new("Continuity", None::<&gtk4::Widget>);
        container.append(&header.container);

        let on_back = Rc::new(on_back);
        header.connect_back(move || {
            on_back();
        });

        // Status section
        let status_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        status_box.set_margin_start(12);
        status_box.set_margin_end(12);
        status_box.set_margin_top(4);

        let status_label = gtk4::Label::builder()
            .label("Status: Inaktiv")
            .halign(gtk4::Align::Start)
            .css_classes(vec!["list-sublabel".to_string()])
            .build();
        status_box.append(&status_label);

        let role_label = gtk4::Label::builder()
            .label("")
            .halign(gtk4::Align::Start)
            .css_classes(vec!["list-sublabel".to_string()])
            .build();
        status_box.append(&role_label);

        container.append(&status_box);

        // PIN confirmation section (hidden by default)
        let pin_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        pin_box.set_margin_start(12);
        pin_box.set_margin_end(12);
        pin_box.set_visible(false);

        let pin_label = gtk4::Label::builder()
            .label("PIN-Bestätigung")
            .halign(gtk4::Align::Start)
            .build();
        pin_box.append(&pin_label);

        let pin_value = gtk4::Label::builder()
            .css_classes(vec!["pin-display".to_string()])
            .halign(gtk4::Align::Center)
            .build();
        pin_box.append(&pin_value);

        let pin_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        pin_actions.set_halign(gtk4::Align::End);

        let reject_btn = gtk4::Button::with_label("Ablehnen");
        reject_btn.add_css_class("destructive-action");
        let tx_reject = ctx.continuity.tx.clone();
        reject_btn.connect_clicked(move |_| {
            let _ = tx_reject.try_send(ContinuityCmd::RejectPin);
        });

        let confirm_btn = gtk4::Button::with_label("Bestätigen");
        confirm_btn.add_css_class("suggested-action");
        let tx_confirm = ctx.continuity.tx.clone();
        confirm_btn.connect_clicked(move |_| {
            let _ = tx_confirm.try_send(ContinuityCmd::ConfirmPin);
        });

        pin_actions.append(&reject_btn);
        pin_actions.append(&confirm_btn);
        pin_box.append(&pin_actions);

        container.append(&pin_box);

        // Peer list
        let scrolled_list = ScrolledList::new(300);
        scrolled_list.list.add_css_class("qs-list");
        container.append(&scrolled_list.scrolled);

        // Disconnect button (hidden by default)
        let disconnect_btn = gtk4::Button::with_label("Trennen");
        disconnect_btn.set_margin_start(12);
        disconnect_btn.set_margin_end(12);
        disconnect_btn.add_css_class("destructive-action");
        disconnect_btn.set_visible(false);
        let tx_disc = ctx.continuity.tx.clone();
        disconnect_btn.connect_clicked(move |_| {
            let _ = tx_disc.try_send(ContinuityCmd::Disconnect);
        });
        container.append(&disconnect_btn);

        // Subscribe to service data
        let list_c = scrolled_list.list;
        let tx_connect = ctx.continuity.tx.clone();
        let status_label_c = status_label.clone();
        let role_label_c = role_label.clone();
        let pin_box_c = pin_box.clone();
        let pin_value_c = pin_value.clone();
        let disconnect_btn_c = disconnect_btn.clone();

        ctx.continuity.subscribe(move |data| {
            // Update status
            if !data.enabled {
                status_label_c.set_label("Status: Deaktiviert");
                role_label_c.set_label("");
            } else if let Some(conn) = &data.active_connection {
                status_label_c.set_label(&format!("Verbunden mit {}", conn.peer_name));
                match data.sharing_mode {
                    SharingMode::Idle => {
                        role_label_c.set_label("Cursor ist lokal");
                    }
                    SharingMode::Sharing => {
                        role_label_c.set_label("Cursor ist auf Remote");
                    }
                    SharingMode::Receiving => {
                        role_label_c.set_label("Remote-Cursor aktiv");
                    }
                }
            } else {
                status_label_c.set_label("Status: Bereit");
                role_label_c.set_label("");
            }

            // PIN confirmation
            if let Some(pending) = &data.pending_pin {
                pin_box_c.set_visible(true);
                pin_value_c.set_label(&pending.pin);
            } else {
                pin_box_c.set_visible(false);
            }

            // Disconnect button
            disconnect_btn_c.set_visible(data.active_connection.is_some());

            // Peer list
            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }

            if !data.enabled {
                let label = gtk4::Label::builder()
                    .label("Continuity ist deaktiviert")
                    .css_classes(vec!["list-sublabel".to_string()])
                    .margin_top(20)
                    .build();
                list_c.append(&label);
                return;
            }

            for peer in &data.peers {
                let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
                row.set_margin_start(12);
                row.set_margin_end(12);
                row.set_margin_top(8);
                row.set_margin_bottom(8);

                let icon = gtk4::Image::from_icon_name("computer-symbolic");
                icon.set_pixel_size(24);
                icon.set_valign(gtk4::Align::Center);
                row.append(&icon);

                let info = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
                info.set_hexpand(true);
                info.set_valign(gtk4::Align::Center);

                let name = gtk4::Label::builder()
                    .label(&peer.device_name)
                    .halign(gtk4::Align::Start)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .build();
                info.append(&name);

                let addr_label = gtk4::Label::builder()
                    .label(&peer.address.to_string())
                    .halign(gtk4::Align::Start)
                    .css_classes(vec!["list-sublabel".to_string()])
                    .build();
                info.append(&addr_label);

                row.append(&info);

                // Connect button
                let is_connected = data
                    .active_connection
                    .as_ref()
                    .is_some_and(|c| c.peer_id == peer.device_id);

                if !is_connected && data.active_connection.is_none() {
                    let connect_btn = gtk4::Button::from_icon_name("network-transmit-receive-symbolic");
                    connect_btn.set_tooltip_text(Some("Verbinden"));
                    connect_btn.add_css_class("flat");
                    connect_btn.add_css_class("circular");
                    let tx_c = tx_connect.clone();
                    let id = peer.device_id.clone();
                    connect_btn.connect_clicked(move |_| {
                        let _ = tx_c.try_send(ContinuityCmd::ConnectToPeer(id.clone()));
                    });
                    row.append(&connect_btn);
                } else if is_connected {
                    let connected_icon = gtk4::Image::from_icon_name("object-select-symbolic");
                    connected_icon.set_tooltip_text(Some("Verbunden"));
                    row.append(&connected_icon);
                }

                let frame = gtk4::Frame::new(None);
                frame.add_css_class("list-row");
                frame.set_child(Some(&row));
                list_c.append(&frame);
            }

            if data.peers.is_empty() {
                let label = gtk4::Label::builder()
                    .label("Keine Axis-Geräte im Netzwerk gefunden")
                    .css_classes(vec!["list-sublabel".to_string()])
                    .margin_top(20)
                    .build();
                list_c.append(&label);
            }
        });

        Self { container }
    }
}

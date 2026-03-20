use crate::app_context::AppContext;
use crate::services::network::NetworkCmd;
use crate::widgets::components::scrolled_list::ScrolledList;
use crate::widgets::icons::wifi_signal_icon;
use crate::widgets::quick_settings::components::header::QsSubPageHeader;
use crate::widgets::quick_settings::components::tile::QsTile;
use crate::widgets::ListRow;
use gtk4::prelude::*;
use std::rc::Rc;

fn build_auth_revealer(
    ap_path: &str,
    ap_ssid: &str,
    tx: &async_channel::Sender<NetworkCmd>,
) -> gtk4::Revealer {
    let revealer = gtk4::Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::SlideDown)
        .transition_duration(200)
        .build();

    let auth_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    auth_box.set_margin_start(12);
    auth_box.set_margin_end(12);
    auth_box.set_margin_bottom(12);
    auth_box.add_css_class("qs-wifi-auth");

    let pass_entry = gtk4::PasswordEntry::builder()
        .placeholder_text("Passwort")
        .hexpand(true)
        .show_peek_icon(true)
        .build();

    let connect_btn = gtk4::Button::builder()
        .label("Verbinden")
        .css_classes(vec![
            "suggested-action".to_string(),
            "qs-wifi-connect-btn".to_string(),
        ])
        .build();

    auth_box.append(&pass_entry);
    auth_box.append(&connect_btn);
    revealer.set_child(Some(&auth_box));

    let tx = tx.clone();
    let ap_path = ap_path.to_string();
    let ap_ssid = ap_ssid.to_string();
    let pass_entry = pass_entry.clone();
    let btn_c = connect_btn.clone();

    connect_btn.connect_clicked(move |_| {
        let password = pass_entry.text().to_string();
        if password.is_empty() {
            return;
        }

        let spinner = gtk4::Spinner::builder()
            .spinning(true)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();
        btn_c.set_child(Some(&spinner));
        btn_c.set_sensitive(false);

        let _ = tx.send_blocking(NetworkCmd::ConnectToApWithPassword(
            ap_path.clone(),
            ap_ssid.clone(),
            password,
        ));
    });

    revealer
}

fn build_ap_row(
    ap: &crate::services::network::AccessPointData,
    tx: &async_channel::Sender<NetworkCmd>,
) -> gtk4::Box {
    let icon = if ap.is_active {
        "network-wireless-connected-symbolic"
    } else if ap.needs_auth {
        "network-wireless-encrypted-symbolic"
    } else {
        wifi_signal_icon(ap.strength)
    };

    let sublabel = if ap.is_active {
        Some("Verbunden")
    } else if ap.needs_auth {
        Some("Gesichert")
    } else {
        None
    };

    let row = ListRow::new(&ap.ssid, icon, ap.is_active, sublabel, false);

    let auth_revealer = build_auth_revealer(&ap.path, &ap.ssid, tx);
    row.container.append(&auth_revealer);

    let tx = tx.clone();
    let path = ap.path.clone();
    let is_active = ap.is_active;
    let needs_auth = ap.needs_auth;
    let revealer = auth_revealer.clone();
    let container_c = row.container.clone();

    row.button.connect_clicked(move |_| {
        if is_active {
            let _ = tx.send_blocking(NetworkCmd::DisconnectWifi);
        } else if needs_auth {
            let open = revealer.reveals_child();
            revealer.set_reveal_child(!open);
            if open {
                container_c.remove_css_class("expanded");
            } else {
                container_c.add_css_class("expanded");
            }
        } else {
            let _ = tx.send_blocking(NetworkCmd::ConnectToAp(path.clone()));
        }
    });

    row.container
}

pub struct WifiPage {
    pub container: gtk4::Box,
}

impl WifiPage {
    pub fn new(
        ctx: AppContext,
        on_back: impl Fn() + 'static,
        wifi_tile: Rc<QsTile>,
        eth_tile: Rc<QsTile>,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        let header = QsSubPageHeader::new("Wi-Fi Netzwerke");
        container.append(&header.container);

        let scrolled_list = ScrolledList::new(300);
        scrolled_list.list.add_css_class("qs-list");
        container.append(&scrolled_list.scrolled);

        // --- LOGIC ---
        let on_back = Rc::new(on_back);
        header.connect_back(move || on_back());

        let list_c = scrolled_list.list;
        let wifi_tile_c = wifi_tile.clone();
        let eth_tile_c = eth_tile.clone();
        let tx = ctx.network_tx.clone();

        ctx.network.subscribe(move |data| {
            wifi_tile_c.set_active(data.is_wifi_enabled);
            eth_tile_c.set_active(data.is_ethernet_connected);

            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }

            for ap in &data.access_points {
                let list_row = gtk4::ListBoxRow::builder()
                    .css_classes(vec!["qs-wifi-item".to_string()])
                    .selectable(false)
                    .activatable(false)
                    .build();
                list_row.set_child(Some(&build_ap_row(ap, &tx)));
                list_c.append(&list_row);
            }

            if data.access_points.is_empty() && data.is_wifi_enabled {
                let empty_label = gtk4::Label::builder()
                    .label("Suche nach Netzwerken...")
                    .css_classes(vec!["list-sublabel".to_string()])
                    .margin_top(20)
                    .build();
                list_c.append(&empty_label);
            }
        });

        Self { container }
    }
}

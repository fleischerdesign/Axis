use crate::app_context::AppContext;
use crate::services::network::NetworkCmd;
use crate::widgets::quick_settings::components::{QsListRow, QsTile};
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct WifiPage {
    pub container: gtk4::Box,
}

impl WifiPage {
    pub fn new(
        ctx: AppContext,
        back_callback: impl Fn() + 'static,
        wifi_tile: Rc<QsTile>,
        eth_tile: Rc<QsTile>,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 16);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        let back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["qs-back-btn".to_string()])
            .build();
        let title = gtk4::Label::builder()
            .label("Wi-Fi")
            .halign(gtk4::Align::Start)
            .css_classes(vec!["qs-subpage-title".to_string()])
            .build();
        header.append(&back_btn);
        header.append(&title);

        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list".to_string()])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();
        scrolled.add_css_class("qs-scrolled");
        scrolled.set_child(Some(&list));

        container.append(&header);
        container.append(&scrolled);
        container.set_vexpand(true);

        back_btn.connect_clicked(move |_| back_callback());

        // State-Tracking: AP-Liste nur neu aufbauen wenn sich was geändert hat
        let last_ap_ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        ctx.network.subscribe(move |data| {
            // Tile-States aktualisieren
            wifi_tile.set_active(data.is_wifi_enabled);
            eth_tile.set_active(data.is_ethernet_connected);

            let wifi_icon = if !data.is_wifi_enabled || !data.is_wifi_connected {
                "network-wireless-offline-symbolic"
            } else if data.active_strength > 80 {
                "network-wireless-signal-excellent-symbolic"
            } else if data.active_strength > 60 {
                "network-wireless-signal-good-symbolic"
            } else if data.active_strength > 40 {
                "network-wireless-signal-ok-symbolic"
            } else {
                "network-wireless-signal-weak-symbolic"
            };
            wifi_tile.set_icon(wifi_icon);

            // AP-Liste nur neu aufbauen wenn sich die IDs geändert haben
            let current_ap_ids: Vec<String> = data
                .access_points
                .iter()
                .map(|ap| format!("{}-{}", ap.ssid, ap.is_active))
                .collect();

            if *last_ap_ids.borrow() == current_ap_ids {
                return;
            }

            // Nicht rebuilden wenn ein Passwort-Feld offen ist
            let any_expanded = {
                let mut expanded = false;
                let mut curr = list.first_child();
                while let Some(row) = curr {
                    if let Some(item_box) = row.downcast_ref::<gtk4::Box>() {
                        if let Some(revealer) = item_box
                            .last_child()
                            .and_then(|c| c.downcast::<gtk4::Revealer>().ok())
                        {
                            if revealer.reveals_child() {
                                expanded = true;
                                break;
                            }
                        }
                    }
                    curr = row.next_sibling();
                }
                expanded
            };

            if any_expanded {
                return;
            }

            *last_ap_ids.borrow_mut() = current_ap_ids;

            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            for ap in &data.access_points {
                let icon = if ap.strength > 75 {
                    "network-wireless-signal-excellent-symbolic"
                } else if ap.strength > 50 {
                    "network-wireless-signal-good-symbolic"
                } else if ap.strength > 25 {
                    "network-wireless-signal-ok-symbolic"
                } else {
                    "network-wireless-signal-weak-symbolic"
                };
                let row = QsListRow::new(&ap.ssid, icon, ap.is_active, None);

                let auth_revealer = gtk4::Revealer::new();
                let auth_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                auth_box.set_margin_start(12);
                auth_box.set_margin_end(12);
                auth_box.set_margin_bottom(12);
                let pass_entry = gtk4::PasswordEntry::builder()
                    .placeholder_text("Password")
                    .hexpand(true)
                    .build();
                let connect_btn = gtk4::Button::builder()
                    .label("Connect")
                    .css_classes(vec![
                        "suggested-action".to_string(),
                        "qs-wifi-connect-btn".to_string(),
                    ])
                    .build();
                auth_box.append(&pass_entry);
                auth_box.append(&connect_btn);
                auth_revealer.set_child(Some(&auth_box));

                let tx = ctx.network_tx.clone();
                let ap_path = ap.path.clone();
                let is_active = ap.is_active;
                let needs_auth = ap.needs_auth;
                let rev = auth_revealer.clone();
                row.button.connect_clicked(move |_| {
                    if is_active {
                        let _ = tx.send_blocking(NetworkCmd::DisconnectWifi);
                    } else if needs_auth {
                        rev.set_reveal_child(!rev.reveals_child());
                    } else {
                        let _ = tx.send_blocking(NetworkCmd::ConnectToAp(ap_path.clone()));
                    }
                });

                let tx = ctx.network_tx.clone();
                let ap_path = ap.path.clone();
                let ap_ssid = ap.ssid.clone();
                let btn_c = connect_btn.clone();
                let pass_c = pass_entry.clone();
                connect_btn.connect_clicked(move |_| {
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
                        pass_c.text().to_string(),
                    ));
                });

                let item_container = row.container;
                item_container.append(&auth_revealer);
                list.append(&item_container);
            }
        });

        Self { container }
    }
}

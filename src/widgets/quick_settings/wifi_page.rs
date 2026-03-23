use crate::app_context::AppContext;
use crate::services::network::NetworkCmd;
use crate::widgets::components::scrolled_list::ScrolledList;
use crate::widgets::components::subpage_header::SubPageHeader;
use crate::widgets::icons::wifi_signal_icon;
use crate::widgets::ListRow;
use crate::widgets::ToggleTile;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct RowEntry {
    list_row: ListRow,
    list_box_row: gtk4::ListBoxRow,
    auth_revealer: gtk4::Revealer,
    path: String,
}

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
            .spinning(false)
            .css_classes(vec!["subpage-spinner".to_string()])
            .build();
        btn_c.set_child(Some(&spinner));
        btn_c.set_sensitive(false);

        let _ = tx.try_send(NetworkCmd::ConnectToApWithPassword(
            ap_path.clone(),
            ap_ssid.clone(),
            password,
        ));
    });

    revealer
}

fn ap_icon(ap: &crate::services::network::AccessPointData) -> &'static str {
    if ap.is_active {
        "network-wireless-connected-symbolic"
    } else if ap.needs_auth {
        "network-wireless-encrypted-symbolic"
    } else {
        wifi_signal_icon(ap.strength)
    }
}

fn ap_sublabel(ap: &crate::services::network::AccessPointData) -> Option<&'static str> {
    if ap.is_active {
        Some("Verbunden")
    } else if ap.needs_auth {
        Some("Gesichert")
    } else {
        None
    }
}

pub struct WifiPage {
    pub container: gtk4::Box,
}

impl WifiPage {
    pub fn new(
        ctx: AppContext,
        on_back: impl Fn() + 'static,
        wifi_tile: Rc<ToggleTile>,
        eth_tile: Rc<ToggleTile>,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        let spinner = gtk4::Spinner::builder().spinning(true).build();

        let header = SubPageHeader::new("Wi-Fi Netzwerke", Some(&spinner));
        container.append(&header.container);

        let scrolled_list = ScrolledList::new(300);
        scrolled_list.list.add_css_class("qs-list");
        container.append(&scrolled_list.scrolled);

        // --- Logic ---
        let on_back = Rc::new(on_back);
        header.connect_back(move || on_back());

        let list_c = scrolled_list.list;
        let wifi_tile_c = wifi_tile.clone();
        let eth_tile_c = eth_tile.clone();
        let tx = ctx.network.tx.clone();

        let rows: Rc<RefCell<HashMap<String, RowEntry>>> = Rc::new(RefCell::new(HashMap::new()));

        let rows_c = rows.clone();
        let spinner_c = spinner;

        ctx.network.subscribe(move |data| {
            wifi_tile_c.set_active(data.is_wifi_enabled);
            eth_tile_c.set_active(data.is_ethernet_connected);
            spinner_c.set_spinning(data.is_scanning);

            let mut rows = rows_c.borrow_mut();

            let new_paths: std::collections::HashSet<&str> = data
                .access_points
                .iter()
                .map(|ap| ap.path.as_str())
                .collect();

            // Remove rows for APs that no longer exist
            let stale: Vec<String> = rows
                .keys()
                .filter(|p| !new_paths.contains(p.as_str()))
                .cloned()
                .collect();
            for path in stale {
                if let Some(entry) = rows.remove(&path) {
                    list_c.remove(&entry.list_box_row);
                }
            }

            for ap in &data.access_points {
                let icon = ap_icon(ap);
                let sublabel = ap_sublabel(ap);

                if let Some(entry) = rows.get(&ap.path) {
                    // Existing row: update content
                    entry
                        .list_row
                        .update(&ap.ssid, icon, ap.is_active, sublabel, false);
                    continue;
                }

                // New row
                let list_row = ListRow::new(&ap.ssid, icon, ap.is_active, sublabel, false);

                let auth_revealer = build_auth_revealer(&ap.path, &ap.ssid, &tx);
                list_row.container.append(&auth_revealer);

                let tx_click = tx.clone();
                let path = ap.path.clone();
                let needs_auth = ap.needs_auth;
                let revealer = auth_revealer.clone();
                let container_c = list_row.container.clone();
                list_row.button.connect_clicked(move |btn| {
                    let connected = btn.has_css_class("active");
                    if connected {
                        let _ = tx_click.try_send(NetworkCmd::DisconnectWifi);
                    } else if needs_auth {
                        let open = revealer.reveals_child();
                        revealer.set_reveal_child(!open);
                        if open {
                            container_c.remove_css_class("expanded");
                        } else {
                            container_c.add_css_class("expanded");
                        }
                    } else {
                        let _ = tx_click.try_send(NetworkCmd::ConnectToAp(path.clone()));
                    }
                });

                let list_box_row = gtk4::ListBoxRow::builder()
                    .css_classes(vec!["qs-wifi-item".to_string()])
                    .selectable(false)
                    .activatable(false)
                    .child(&list_row.container)
                    .build();

                rows.insert(
                    ap.path.clone(),
                    RowEntry {
                        list_row,
                        list_box_row,
                        auth_revealer,
                        path: ap.path.clone(),
                    },
                );
                list_c.append(&rows[&ap.path].list_box_row);
            }
        });

        Self { container }
    }
}

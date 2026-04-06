use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::network_proxy::NetworkProxy;
use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;

pub struct NetworkPage {
    proxy: Option<Rc<NetworkProxy>>,
}

struct RowEntry {
    row: libadwaita::ExpanderRow,
    pass_entry: gtk4::PasswordEntry,
    connect_btn: gtk4::Button,
    is_active: bool,
}

impl NetworkPage {
    pub fn new(proxy: Option<&Rc<NetworkProxy>>) -> Self {
        Self {
            proxy: proxy.cloned(),
        }
    }
}

fn wifi_icon(ap: &axis_core::services::network::dbus::DbusAccessPoint) -> &'static str {
    if ap.is_active {
        "network-wireless-connected-symbolic"
    } else if ap.needs_auth {
        "network-wireless-encrypted-symbolic"
    } else {
        wifi_signal_icon(ap.strength)
    }
}

fn wifi_subtitle(ap: &axis_core::services::network::dbus::DbusAccessPoint) -> String {
    if ap.is_active {
        "Verbunden".to_string()
    } else if ap.needs_auth {
        "Gesichert".to_string()
    } else {
        "".to_string()
    }
}

fn wifi_signal_icon(strength: u8) -> &'static str {
    if strength >= 75 {
        "network-wireless-signal-excellent-symbolic"
    } else if strength >= 50 {
        "network-wireless-signal-good-symbolic"
    } else if strength >= 25 {
        "network-wireless-signal-ok-symbolic"
    } else {
        "network-wireless-signal-weak-symbolic"
    }
}

impl SettingsPage for NetworkPage {
    fn id(&self) -> &'static str { "network" }
    fn title(&self) -> &'static str { "Netzwerk" }
    fn icon(&self) -> &'static str { "network-wireless-symbolic" }

    fn build(&self, _proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        let page = libadwaita::PreferencesPage::new();

        if let Some(ref n) = self.proxy {
            let n_c = n.clone();

            // Initial scan when opening the page
            let n_scan = n.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = n_scan.scan_wifi().await;
            });

            // --- Wi-Fi Toggle ---
            let wifi_group = libadwaita::PreferencesGroup::builder()
                .title("Wi-Fi")
                .build();
            let wifi_switch = libadwaita::SwitchRow::builder()
                .title("Wi-Fi")
                .build();
            wifi_group.add(&wifi_switch);
            page.add(&wifi_group);

            let n_toggle = n.clone();
            wifi_switch.connect_active_notify(move |row| {
                let n = n_toggle.clone();
                let active = row.is_active();
                gtk4::glib::spawn_future_local(async move {
                    let _ = n.set_wifi_enabled(active).await;
                    n.reload();
                });
            });

            // --- Networks ---
            let networks_group = libadwaita::PreferencesGroup::builder()
                .title("Netzwerke")
                .build();
            page.add(&networks_group);

            // --- Ethernet ---
            let eth_group = libadwaita::PreferencesGroup::builder()
                .title("Ethernet")
                .build();
            let eth_status = libadwaita::ActionRow::builder()
                .title("Status")
                .build();
            eth_group.add(&eth_status);
            page.add(&eth_group);

            // Row management
            let rows: Rc<RefCell<HashMap<String, RowEntry>>> = Rc::new(RefCell::new(HashMap::new()));
            let rows_c = rows.clone();

            n.on_change(move || {
                let state = n_c.state();
                let mut rows_map = rows_c.borrow_mut();

                wifi_switch.set_active(state.is_wifi_enabled);
                eth_status.set_subtitle(&if state.is_ethernet_connected { "Verbunden" } else { "Nicht verbunden" });

                let new_paths: std::collections::HashSet<String> = state.access_points.iter().map(|ap| ap.path.clone()).collect();
                
                // Remove stale
                let stale: Vec<String> = rows_map.keys().filter(|p| !new_paths.contains(*p)).cloned().collect();
                for path in stale {
                    if let Some(entry) = rows_map.remove(&path) {
                        networks_group.remove(&entry.row);
                    }
                }

                // Add or Update
                for ap in &state.access_points {
                    if let Some(entry) = rows_map.get_mut(&ap.path) {
                        entry.row.set_title(&ap.ssid);
                        entry.row.set_subtitle(&wifi_subtitle(ap));
                        
                        if ap.is_active && !entry.is_active {
                            entry.row.set_expanded(false);
                            entry.connect_btn.set_child(None::<&gtk4::Widget>);
                            entry.connect_btn.set_label("Verbinden");
                            entry.connect_btn.set_sensitive(true);
                            entry.pass_entry.set_text("");
                        }
                        entry.is_active = ap.is_active;
                        continue;
                    }

                    // New ExpanderRow
                    let row = libadwaita::ExpanderRow::builder()
                        .title(&ap.ssid)
                        .subtitle(wifi_subtitle(ap))
                        .build();

                    let icon = gtk4::Image::from_icon_name(wifi_icon(ap));
                    icon.set_pixel_size(20);
                    row.add_prefix(&icon);

                    // Auth section
                    let auth_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                    auth_box.add_css_class("network-auth-box");
                    auth_box.set_margin_start(12);
                    auth_box.set_margin_end(12);
                    auth_box.set_margin_top(8);
                    auth_box.set_margin_bottom(8);

                    let pass_entry = gtk4::PasswordEntry::builder()
                        .placeholder_text("Passwort")
                        .hexpand(true)
                        .build();
                    
                    let connect_btn = gtk4::Button::builder()
                        .label("Verbinden")
                        .css_classes(vec!["suggested-action".to_string(), "network-connect-btn".to_string()])
                        .build();

                    auth_box.append(&pass_entry);
                    auth_box.append(&connect_btn);

                    if ap.needs_auth {
                        row.add_row(&auth_box);
                    }

                    let n_click = n_c.clone();
                    let ap_path = ap.path.clone();
                    let ap_ssid = ap.ssid.clone();
                    let needs_auth = ap.needs_auth;
                    let is_active = ap.is_active;
                    let pass_c = pass_entry.clone();
                    let btn_c = connect_btn.clone();

                    // Disconnect button for active
                    if is_active {
                        let disconnect_btn = gtk4::Button::builder()
                            .icon_name("window-close-symbolic")
                            .css_classes(vec!["flat".to_string()])
                            .valign(gtk4::Align::Center)
                            .build();
                        row.add_suffix(&disconnect_btn);
                        
                        let n_disconnect = n_c.clone();
                        disconnect_btn.connect_clicked(move |_| {
                            let n = n_disconnect.clone();
                            gtk4::glib::spawn_future_local(async move {
                                let _ = n.disconnect_wifi().await;
                                n.reload();
                            });
                        });
                        row.set_expanded(false);
                    } else if !needs_auth {
                        // Open network: connect on expansion
                        let n_open = n_c.clone();
                        let p_open = ap_path.clone();
                        row.connect_expanded_notify(move |r| {
                            if r.is_expanded() {
                                let n = n_open.clone();
                                let p = p_open.clone();
                                gtk4::glib::spawn_future_local(async move {
                                    let _ = n.connect_ap(&p).await;
                                    n.reload();
                                });
                                r.set_expanded(false);
                            }
                        });
                    }

                    // Connect button
                    let n_connect = n_c.clone();
                    let ap_path_c = ap.path.clone();
                    let ap_ssid_c = ap.ssid.clone();
                    connect_btn.connect_clicked(move |_| {
                        let password = pass_c.text().to_string();
                        if password.is_empty() { return; }

                        let spinner = gtk4::Spinner::builder().spinning(true).build();
                        btn_c.set_child(Some(&spinner));
                        btn_c.set_sensitive(false);

                        let n = n_connect.clone();
                        let p = ap_path_c.clone();
                        let s = ap_ssid_c.clone();
                        gtk4::glib::spawn_future_local(async move {
                            let _ = n.connect_ap_with_password(&p, &s, &password).await;
                            n.reload();
                        });
                    });

                    networks_group.add(&row);
                    rows_map.insert(ap.path.clone(), RowEntry {
                        row,
                        pass_entry,
                        connect_btn,
                        is_active: ap.is_active,
                    });
                }
            });
        }

        page.into()
    }
}

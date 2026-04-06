use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::network_proxy::NetworkProxy;
use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;

pub struct NetworkPage {
    proxy: Option<Rc<NetworkProxy>>,
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

fn wifi_subtitle(ap: &axis_core::services::network::dbus::DbusAccessPoint) -> Option<String> {
    if ap.is_active {
        Some("Verbunden".to_string())
    } else if ap.needs_auth {
        Some("Gesichert".to_string())
    } else {
        None
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

        let np = self.proxy.clone();

        // ── Wi-Fi Toggle ───────────────────────────────────────────────────
        let wifi_group = libadwaita::PreferencesGroup::builder()
            .title("Wi-Fi")
            .build();

        let wifi_switch = libadwaita::SwitchRow::builder()
            .title("Wi-Fi")
            .build();

        if let Some(ref n) = np {
            let state = n.state();
            wifi_switch.set_active(state.is_wifi_enabled);
            
            if state.is_wifi_enabled && state.access_points.is_empty() {
                let n_scan = n.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = n_scan.scan_wifi().await;
                });
            }
        }

        if let Some(ref n) = np {
            let n_inner = n.clone();
            wifi_switch.connect_active_notify(move |row| {
                let n = n_inner.clone();
                let active = row.is_active();
                gtk4::glib::spawn_future_local(async move {
                    let _ = n.set_wifi_enabled(active).await;
                    n.reload();
                });
            });
        }

        let wifi_switch_widget: gtk4::Widget = wifi_switch.into();
        wifi_group.add(&wifi_switch_widget);
        page.add(&wifi_group);

        // ── Networks ────────────────────────────────────────────────────────
        if let Some(ref n) = np {
            let state = n.state();

            let networks_group = libadwaita::PreferencesGroup::builder()
                .title("Netzwerke")
                .build();

            for ap in &state.access_points {
                let row = libadwaita::ActionRow::builder()
                    .title(&ap.ssid)
                    .build();

                let icon = gtk4::Image::from_icon_name(wifi_icon(ap));
                icon.set_pixel_size(20);
                row.add_prefix(&icon);

                if let Some(label) = wifi_subtitle(ap) {
                    row.set_subtitle(&label);
                }

                // Click handler via activatable widget (like QS)
                let click_bridge = gtk4::Button::new();
                click_bridge.set_hexpand(true);
                click_bridge.set_valign(gtk4::Align::Center);
                row.set_activatable_widget(Some(&click_bridge));

                let path = ap.path.clone();
                let ssid = ap.ssid.clone();
                let needs_auth = ap.needs_auth;
                let n_connect = n.clone();
                let is_active = ap.is_active;

                click_bridge.connect_clicked(move |_| {
                    let n = n_connect.clone();
                    if is_active {
                        gtk4::glib::spawn_future_local(async move {
                            let _ = n.disconnect_wifi().await;
                        });
                    } else {
                        let path = path.clone();
                        let ssid = ssid.clone();
                        gtk4::glib::spawn_future_local(async move {
                            if needs_auth {
                                let _ = n.connect_ap_with_password(&path, &ssid, "password").await;
                            } else {
                                let _ = n.connect_ap(&path).await;
                            }
                        });
                    }
                });

                let row_widget: gtk4::Widget = row.into();
                networks_group.add(&row_widget);
            }

            if state.access_points.is_empty() && state.is_wifi_enabled {
                let row = libadwaita::ActionRow::builder()
                    .title("Keine Netzwerke gefunden")
                    .build();
                let row_widget: gtk4::Widget = row.into();
                networks_group.add(&row_widget);
            }

            page.add(&networks_group);
        }

        // ── Ethernet ───────────────────────────────────────────────────────
        let eth_group = libadwaita::PreferencesGroup::builder()
            .title("Ethernet")
            .build();

        let eth_status = libadwaita::ActionRow::builder()
            .title("Status")
            .subtitle("Nicht verbunden")
            .build();

        if let Some(ref n) = np {
            let state = n.state();
            if state.is_ethernet_connected {
                eth_status.set_subtitle("Verbunden");
            }
        }

        let eth_status_widget: gtk4::Widget = eth_status.into();
        eth_group.add(&eth_status_widget);
        page.add(&eth_group);

        page.into()
    }
}

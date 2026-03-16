use gtk4::prelude::*;
use crate::services::network::NetworkCmd;
use crate::app_context::AppContext;
use crate::widgets::quick_settings::components::QsListRow;
use std::rc::Rc;
use std::cell::RefCell;

pub struct WifiPage {
    pub container: gtk4::Box,
}

impl WifiPage {
    pub fn new(
        ctx: AppContext,
        back_callback: impl Fn() + 'static,
        wifi_tile: Rc<crate::widgets::quick_settings::components::QsTile>,
        eth_tile: Rc<crate::widgets::quick_settings::components::QsTile>,
    ) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        let back_btn = gtk4::Button::builder().icon_name("go-previous-symbolic").css_classes(vec!["qs-back-btn".to_string()]).build();
        let title = gtk4::Label::builder().label("Wi-Fi").halign(gtk4::Align::Start).css_classes(vec!["qs-subpage-title".to_string()]).build();
        
        header.append(&back_btn);
        header.append(&title);
        
        let list = gtk4::ListBox::builder().css_classes(vec!["qs-list".to_string()]).selection_mode(gtk4::SelectionMode::None).build();
        container.append(&header);
        container.append(&list);

        back_btn.connect_clicked(move |_| {
            back_callback();
        });

        let list_c = list.clone();
        let ctx_c = ctx.clone();
        let mut network_rx = ctx.network_rx.clone();
        
        // State-Tracking für minimales UI-Rebuilding
        let last_ap_ids = Rc::new(RefCell::new(Vec::<String>::new()));

        gtk4::glib::MainContext::default().spawn_local(async move {
            Self::update_ui(&network_rx.borrow(), &wifi_tile, &eth_tile, &list_c, &ctx_c, &last_ap_ids);

            while network_rx.changed().await.is_ok() {
                Self::update_ui(&network_rx.borrow(), &wifi_tile, &eth_tile, &list_c, &ctx_c, &last_ap_ids);
            }
        });

        Self { container }
    }

    fn update_ui(
        data: &crate::services::network::NetworkData,
        wifi_tile: &Rc<crate::widgets::quick_settings::components::QsTile>,
        eth_tile: &Rc<crate::widgets::quick_settings::components::QsTile>,
        list: &gtk4::ListBox,
        ctx: &AppContext,
        last_ap_ids: &Rc<RefCell<Vec<String>>>,
    ) {
        wifi_tile.set_active(data.is_wifi_enabled);
        eth_tile.set_active(data.is_ethernet_connected);
        
        let wifi_icon = if !data.is_wifi_enabled || !data.is_wifi_connected {
            "network-wireless-offline-symbolic"
        } else if data.active_strength > 80 { "network-wireless-signal-excellent-symbolic" }
        else if data.active_strength > 60 { "network-wireless-signal-good-symbolic" }
        else if data.active_strength > 40 { "network-wireless-signal-ok-symbolic" }
        else { "network-wireless-signal-weak-symbolic" };
        wifi_tile.set_icon(wifi_icon);

        // Prüfen, ob sich die AP-Liste (IDs/SSIDs) geändert hat
        let current_ap_ids: Vec<String> = data.access_points.iter().map(|ap| format!("{}-{}", ap.ssid, ap.is_active)).collect();
        let mut ids_changed = false;
        {
            let last = last_ap_ids.borrow();
            if *last != current_ap_ids {
                ids_changed = true;
            }
        }

        if !ids_changed {
            return; // Nichts zu tun, wenn die APs identisch sind
        }

        let mut any_expanded = false;
        let mut curr = list.first_child();
        while let Some(row) = curr {
            if let Some(item_box) = row.downcast_ref::<gtk4::Box>() {
                if let Some(revealer) = item_box.last_child().and_then(|c| c.downcast::<gtk4::Revealer>().ok()) {
                    if revealer.reveals_child() { any_expanded = true; break; }
                }
            }
            curr = row.next_sibling();
        }

        if !any_expanded {
            *last_ap_ids.borrow_mut() = current_ap_ids;
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            for ap in &data.access_points {
                let icon = if ap.strength > 75 { "network-wireless-signal-excellent-symbolic" } else if ap.strength > 50 { "network-wireless-signal-good-symbolic" } else if ap.strength > 25 { "network-wireless-signal-ok-symbolic" } else { "network-wireless-signal-weak-symbolic" };
                let row = QsListRow::new(&ap.ssid, icon, ap.is_active, None);
                
                let auth_revealer = gtk4::Revealer::new();
                let auth_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                auth_box.set_margin_start(12); auth_box.set_margin_end(12); auth_box.set_margin_bottom(12);
                let pass_entry = gtk4::PasswordEntry::builder().placeholder_text("Password").hexpand(true).build();
                let connect_btn = gtk4::Button::builder().label("Connect").css_classes(vec!["suggested-action".to_string(), "qs-wifi-connect-btn".to_string()]).build();
                auth_box.append(&pass_entry); auth_box.append(&connect_btn);
                auth_revealer.set_child(Some(&auth_box));

                let tx_row = ctx.network_tx.clone();
                let ap_path = ap.path.clone();
                let is_active = ap.is_active;
                let needs_auth = ap.needs_auth;
                let rev = auth_revealer.clone();
                row.button.connect_clicked(move |_| {
                    if is_active { let _ = tx_row.unbounded_send(NetworkCmd::DisconnectWifi); }
                    else if needs_auth { rev.set_reveal_child(!rev.reveals_child()); }
                    else { let _ = tx_row.unbounded_send(NetworkCmd::ConnectToAp(ap_path.clone())); }
                });

                let tx_connect = ctx.network_tx.clone();
                let ap_path_connect = ap.path.clone();
                let ap_ssid_connect = ap.ssid.clone();
                let btn_c = connect_btn.clone();
                let pass_entry_c = pass_entry.clone();
                connect_btn.connect_clicked(move |_| {
                    let spinner = gtk4::Spinner::builder().spinning(true).halign(gtk4::Align::Center).valign(gtk4::Align::Center).build();
                    btn_c.set_child(Some(&spinner)); btn_c.set_sensitive(false);
                    let _ = tx_connect.unbounded_send(NetworkCmd::ConnectToApWithPassword(ap_path_connect.clone(), ap_ssid_connect.clone(), pass_entry_c.text().to_string()));
                });

                let item_container = row.container;
                item_container.append(&auth_revealer);
                list.append(&item_container);
            }
        }
    }
}

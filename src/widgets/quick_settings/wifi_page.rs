use crate::app_context::AppContext;
use crate::services::network::NetworkCmd;
use crate::widgets::quick_settings::components::tile::QsTile;
use crate::widgets::ListRow;
use gtk4::prelude::*;
use std::rc::Rc;

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

        // --- HEADER ---
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["qs-back-btn".to_string()])
            .build();
        
        let title = gtk4::Label::builder()
            .label("Wi-Fi Netzwerke")
            .halign(gtk4::Align::Start)
            .css_classes(vec!["qs-subpage-title".to_string()])
            .build();
        
        header.append(&back_btn);
        header.append(&title);
        container.append(&header);

        // --- LIST ---
        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list".to_string()])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .min_content_height(300)
            .build();
        scrolled.set_child(Some(&list));
        container.append(&scrolled);

        // --- LOGIC ---
        let on_back = Rc::new(on_back);
        let on_back_c = on_back.clone();
        back_btn.connect_clicked(move |_| {
            on_back_c();
        });

        let list_c = list.clone();
        let wifi_tile_c = wifi_tile.clone();
        let eth_tile_c = eth_tile.clone();
        let tx_row = ctx.network_tx.clone();

        ctx.network.subscribe(move |data| {
            wifi_tile_c.set_active(data.is_wifi_enabled);
            eth_tile_c.set_active(data.is_ethernet_connected);
            
            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }

            for ap in &data.access_points {
                let row = ListRow::new(
                    &ap.ssid,
                    if ap.needs_auth { "network-wireless-encrypted-symbolic" } else { "network-wireless-signal-excellent-symbolic" },
                    ap.is_active,
                    if ap.is_active { Some("Verbunden") } else { None },
                    false,
                );

                let tx_inner = tx_row.clone();
                let path_inner = ap.path.clone();

                row.button.connect_clicked(move |_| {
                    let _ = tx_inner.send_blocking(NetworkCmd::ConnectToAp(path_inner.clone()));
                });

                list_c.append(&row.container);
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

use crate::app_context::AppContext;
use crate::services::bluetooth::BluetoothCmd;
use crate::widgets::quick_settings::components::tile::QsTile;
use crate::widgets::ListRow;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct BluetoothPage {
    pub container: gtk4::Box,
}

impl BluetoothPage {
    pub fn new(ctx: AppContext, on_back: impl Fn() + 'static, parent_tile: Rc<QsTile>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        // --- HEADER ---
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["qs-back-btn".to_string()])
            .build();
        
        let title = gtk4::Label::builder()
            .label("Bluetooth")
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
            .min_content_height(300) // Sicherstellen, dass die Liste Platz hat
            .build();
        scrolled.set_child(Some(&list));
        container.append(&scrolled);

        // --- LOGIC ---
        let tx_back = ctx.bluetooth_tx.clone();
        let on_back = Rc::new(on_back);
        let on_back_c = on_back.clone();
        back_btn.connect_clicked(move |_| {
            let _ = tx_back.send_blocking(BluetoothCmd::StopScan);
            on_back_c();
        });

        let list_c = list.clone();
        let parent_tile_c = parent_tile.clone();
        let tx_row = ctx.bluetooth_tx.clone();

        ctx.bluetooth.subscribe(move |data| {
            parent_tile_c.set_active(data.is_powered);
            
            // Liste leeren
            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }

            // Geräte rendern
            for device in &data.devices {
                let row = ListRow::new(
                    &device.name,
                    &device.icon,
                    device.is_connected,
                    if device.is_connected { Some("Verbunden") } else if device.is_paired { Some("Gekoppelt") } else { None },
                    false,
                );

                let tx_inner = tx_row.clone();
                let path_inner = device.path.clone();
                let is_connected = device.is_connected;

                row.button.connect_clicked(move |_| {
                    if is_connected {
                        let _ = tx_inner.send_blocking(BluetoothCmd::Disconnect(path_inner.clone()));
                    } else {
                        let _ = tx_inner.send_blocking(BluetoothCmd::Connect(path_inner.clone()));
                    }
                });

                list_c.append(&row.container);
            }

            if data.devices.is_empty() && data.is_powered {
                let empty_label = gtk4::Label::builder()
                    .label("Suche nach Geräten...")
                    .css_classes(vec!["list-sublabel".to_string()])
                    .margin_top(20)
                    .build();
                list_c.append(&empty_label);
            }
        });

        Self { container }
    }
}

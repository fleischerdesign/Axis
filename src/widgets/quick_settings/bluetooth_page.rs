use crate::app_context::AppContext;
use crate::services::bluetooth::BluetoothCmd;
use crate::widgets::components::scrolled_list::ScrolledList;
use crate::widgets::components::subpage_header::SubPageHeader;
use crate::widgets::ListRow;
use crate::widgets::ToggleTile;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct BluetoothPage {
    pub container: gtk4::Box,
}

impl BluetoothPage {
    pub fn new(ctx: AppContext, on_back: impl Fn() + 'static, parent_tile: Rc<ToggleTile>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        let header = SubPageHeader::new("Bluetooth");
        container.append(&header.container);

        let scrolled_list = ScrolledList::new(300);
        scrolled_list.list.add_css_class("qs-list");
        container.append(&scrolled_list.scrolled);

        // --- LOGIC ---
        let tx_back = ctx.bluetooth.tx.clone();
        let on_back = Rc::new(on_back);
        header.connect_back(move || {
            let _ = tx_back.try_send(BluetoothCmd::StopScan);
            on_back();
        });

        let list_c = scrolled_list.list;
        let parent_tile_c = parent_tile.clone();
        let tx_row = ctx.bluetooth.tx.clone();

        ctx.bluetooth.subscribe(move |data| {
            parent_tile_c.set_active(data.is_powered);

            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }

            for device in &data.devices {
                let row = ListRow::new(
                    &device.name,
                    &device.icon,
                    device.is_connected,
                    if device.is_connected {
                        Some("Verbunden")
                    } else if device.is_paired {
                        Some("Gekoppelt")
                    } else {
                        None
                    },
                    false,
                );

                let tx_inner = tx_row.clone();
                let path_inner = device.path.clone();
                let is_connected = device.is_connected;

                row.button.connect_clicked(move |_| {
                    if is_connected {
                        let _ = tx_inner.try_send(BluetoothCmd::Disconnect(path_inner.clone()));
                    } else {
                        let _ = tx_inner.try_send(BluetoothCmd::Connect(path_inner.clone()));
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

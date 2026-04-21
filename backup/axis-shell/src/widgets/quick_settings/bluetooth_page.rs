use crate::app_context::AppContext;
use axis_core::services::bluetooth::BluetoothCmd;
use crate::widgets::components::scrolled_list::ScrolledList;
use crate::widgets::components::subpage_header::SubPageHeader;
use crate::widgets::ListRow;
use crate::widgets::ToggleTile;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct RowEntry {
    list_row: ListRow,
    list_box_row: gtk4::ListBoxRow,
}

static DEVICE_PATH_KEY: &str = "device-path";

pub struct BluetoothPage {
    pub container: gtk4::Box,
}

impl BluetoothPage {
    pub fn new(ctx: AppContext, on_back: impl Fn() + 'static, parent_tile: Rc<ToggleTile>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);

        let header = SubPageHeader::new("Bluetooth", None::<&gtk4::Widget>);
        container.append(&header.container);

        let scrolled_list = ScrolledList::with_default_height();
        scrolled_list.list.add_css_class("qs-list");
        container.append(&scrolled_list.scrolled);

        let tx_back = ctx.bluetooth.tx.clone();
        let on_back = Rc::new(on_back);
        header.connect_back(move || {
            let _ = tx_back.try_send(BluetoothCmd::StopScan);
            on_back();
        });

        let list_c = scrolled_list.list;
        let parent_tile_c = parent_tile.clone();
        let tx_row = ctx.bluetooth.tx.clone();

        let rows: Rc<RefCell<HashMap<String, RowEntry>>> = Rc::new(RefCell::new(HashMap::new()));

        // Sort by device order — map updated before each invalidate_sort
        let sort_order: Rc<RefCell<HashMap<String, usize>>> = Rc::new(RefCell::new(HashMap::new()));
        let order_for_sort = sort_order.clone();
        list_c.set_sort_func(move |a, b| {
            let order = order_for_sort.borrow();
            let pos_a = unsafe { a.data::<String>(DEVICE_PATH_KEY) }
                .map(|p| unsafe { p.as_ref().clone() })
                .and_then(|p| order.get(&p).copied())
                .unwrap_or(usize::MAX);
            let pos_b = unsafe { b.data::<String>(DEVICE_PATH_KEY) }
                .map(|p| unsafe { p.as_ref().clone() })
                .and_then(|p| order.get(&p).copied())
                .unwrap_or(usize::MAX);
            match pos_a.cmp(&pos_b) {
                std::cmp::Ordering::Less => gtk4::Ordering::Smaller,
                std::cmp::Ordering::Equal => gtk4::Ordering::Equal,
                std::cmp::Ordering::Greater => gtk4::Ordering::Larger,
            }
        });

        let rows_c = rows.clone();

        ctx.bluetooth.subscribe(move |data| {
            parent_tile_c.set_active(data.is_powered);

            let mut rows = rows_c.borrow_mut();

            let new_paths: std::collections::HashSet<&str> =
                data.devices.iter().map(|d| d.path.as_str()).collect();

            // Remove rows for devices that no longer exist
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

            for (i, device) in data.devices.iter().enumerate() {
                let sublabel = if device.is_connected {
                    Some("Verbunden")
                } else if device.is_paired {
                    Some("Gekoppelt")
                } else {
                    None
                };

                if let Some(entry) = rows.get(&device.path) {
                    // Existing row: update content only
                    entry.list_row.update(
                        &device.name,
                        &device.icon,
                        device.is_connected,
                        sublabel,
                        false,
                    );
                } else {
                    // New row: create with click handler (connected once)
                    let list_row = ListRow::new(
                        &device.name,
                        &device.icon,
                        device.is_connected,
                        sublabel,
                        false,
                    );

                    let tx = tx_row.clone();
                    let path = device.path.clone();
                    list_row.button.connect_clicked(move |btn| {
                        let connected = btn.has_css_class("active");
                        let cmd = if connected {
                            BluetoothCmd::Disconnect(path.clone())
                        } else {
                            BluetoothCmd::Connect(path.clone())
                        };
                        let _ = tx.try_send(cmd);
                    });

                    let list_box_row = gtk4::ListBoxRow::builder()
                        .child(&list_row.container)
                        .build();
                    unsafe {
                        list_box_row.set_data(DEVICE_PATH_KEY, device.path.clone());
                    }

                    rows.insert(
                        device.path.clone(),
                        RowEntry {
                            list_row,
                            list_box_row,
                        },
                    );
                    list_c.append(&rows[&device.path].list_box_row);
                }
            }

            // Update sort order and re-sort
            {
                let mut order = sort_order.borrow_mut();
                order.clear();
                for (i, device) in data.devices.iter().enumerate() {
                    order.insert(device.path.clone(), i);
                }
            }
            list_c.invalidate_sort();
        });

        Self { container }
    }
}

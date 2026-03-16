use crate::app_context::AppContext;
use crate::services::bluetooth::BluetoothCmd;
use crate::widgets::quick_settings::components::{QsListRow, QsTile};
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct BluetoothPage {
    pub container: gtk4::Box,
}

impl BluetoothPage {
    pub fn new(ctx: AppContext, back_callback: impl Fn() + 'static, bt_tile: Rc<QsTile>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 16);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
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

        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list".to_string()])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_height(200)
            .max_content_height(500)
            .build();
        scrolled.add_css_class("qs-scrolled");
        scrolled.set_child(Some(&list));

        container.append(&header);
        container.append(&scrolled);

        back_btn.connect_clicked(move |_| back_callback());

        // State-Tracking: Geräteliste nur neu aufbauen wenn sich was geändert hat
        let last_device_ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        ctx.bluetooth.subscribe(move |data| {
            bt_tile.set_active(data.is_powered);

            let current_ids: Vec<String> = data
                .devices
                .iter()
                .map(|d| format!("{}-{}", d.path, d.is_connected))
                .collect();

            if *last_device_ids.borrow() == current_ids {
                return;
            }
            *last_device_ids.borrow_mut() = current_ids;

            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            for dev in &data.devices {
                let sublabel = if dev.is_connected {
                    Some("Connected")
                } else if dev.is_paired {
                    Some("Paired")
                } else {
                    None
                };
                let row = QsListRow::new(&dev.name, &dev.icon, dev.is_connected, sublabel);

                let tx = ctx.bluetooth_tx.clone();
                let path = dev.path.clone();
                let is_connected = dev.is_connected;
                row.button.connect_clicked(move |_| {
                    if is_connected {
                        let _ = tx.send_blocking(BluetoothCmd::Disconnect(path.clone()));
                    } else {
                        let _ = tx.send_blocking(BluetoothCmd::Connect(path.clone()));
                    }
                });

                list.append(&row.container);
            }
        });

        Self { container }
    }
}

use crate::app_context::AppContext;
use crate::services::bluetooth::BluetoothCmd;
use crate::widgets::quick_settings::components::QsListRow;
use gtk4::glib::clone;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct BluetoothPage {
    pub container: gtk4::Box,
}

impl BluetoothPage {
    pub fn new(
        ctx: AppContext,
        back_callback: impl Fn() + 'static,
        bt_tile: Rc<crate::widgets::quick_settings::components::QsTile>,
    ) -> Self {
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
        container.append(&header);
        container.append(&list);

        back_btn.connect_clicked(move |_| {
            back_callback();
        });

        // Autonome Update-Schleife
        let list_c = list.clone();
        let ctx_c = ctx.clone();
        let bt_rx = ctx.bluetooth_rx.clone();

        // State-Tracking
        let last_device_ids = Rc::new(RefCell::new(Vec::<String>::new()));

        gtk4::glib::spawn_future_local(clone!(
            #[strong]
            list_c,
            #[strong]
            bt_tile,
            #[strong]
            ctx_c,
            #[strong]
            last_device_ids,
            async move {
                while let Ok(data) = bt_rx.recv().await {
                    Self::update_ui(&data, &bt_tile, &list_c, &ctx_c, &last_device_ids);
                }
            }
        ));

        Self { container }
    }

    fn update_ui(
        data: &crate::services::bluetooth::BluetoothData,
        bt_tile: &Rc<crate::widgets::quick_settings::components::QsTile>,
        list: &gtk4::ListBox,
        ctx: &AppContext,
        last_device_ids: &Rc<RefCell<Vec<String>>>,
    ) {
        bt_tile.set_active(data.is_powered);

        // Prüfen, ob sich die Liste der Geräte geändert hat
        let current_ids: Vec<String> = data
            .devices
            .iter()
            .map(|d| format!("{}-{}", d.path, d.is_connected))
            .collect();
        {
            let last = last_device_ids.borrow();
            if *last == current_ids {
                return;
            }
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

            let tx_c = ctx.bluetooth_tx.clone();
            let path = dev.path.clone();
            let is_connected = dev.is_connected;
            row.button.connect_clicked(move |_| {
                if is_connected {
                    let _ = tx_c.send_blocking(BluetoothCmd::Disconnect(path.clone()));
                } else {
                    let _ = tx_c.send_blocking(BluetoothCmd::Connect(path.clone()));
                }
            });
            list.append(&row.container);
        }
    }
}

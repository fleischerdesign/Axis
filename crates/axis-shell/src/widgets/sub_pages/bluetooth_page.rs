use gtk4::prelude::*;
use gtk4::glib;
use axis_domain::models::bluetooth::BluetoothStatus;
use axis_presentation::View;
use crate::presentation::bluetooth::{BluetoothPresenter, BluetoothView};
use crate::widgets::components::list_row::ListRow;
use crate::widgets::components::popup_header::PopupHeader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct DeviceEntry {
    list_box_row: gtk4::ListBoxRow,
    list_row: ListRow,
}

pub struct BluetoothPage {
    pub container: gtk4::Box,
    _presenter: Rc<BluetoothPresenter>,
}

impl BluetoothPage {
    pub fn new(presenter: Rc<BluetoothPresenter>, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);

        let header = PopupHeader::new("Bluetooth");
        container.append(&header.container);

        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list".to_string()])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .min_content_height(200)
            .build();
        scrolled.set_child(Some(&list));
        container.append(&scrolled);

        let presenter_c = presenter.clone();
        let on_back = Rc::new(on_back);
        header.connect_back(move || {
            presenter_c.stop_scan();
            on_back();
        });

        presenter.start_scan();

        let rows: Rc<RefCell<HashMap<String, DeviceEntry>>> = Rc::new(RefCell::new(HashMap::new()));
        let rows_c = rows.clone();
        let list_c = list.clone();
        let presenter_c = presenter.clone();

        let view = Box::new(BluetoothPageView {
            rows: rows_c,
            list: list_c,
            presenter: presenter_c,
        });
        presenter.add_view(view);

        Self { container, _presenter: presenter }
    }
}

struct BluetoothPageView {
    rows: Rc<RefCell<HashMap<String, DeviceEntry>>>,
    list: gtk4::ListBox,
    presenter: Rc<BluetoothPresenter>,
}

impl View<BluetoothStatus> for BluetoothPageView {
    fn render(&self, status: &BluetoothStatus) {
        let mut rows = self.rows.borrow_mut();

        let new_ids: std::collections::HashSet<&str> = status
            .devices
            .iter()
            .map(|d| d.id.as_str())
            .collect();

        let stale: Vec<String> = rows
            .keys()
            .filter(|id| !new_ids.contains(id.as_str()))
            .cloned()
            .collect();
        for id in stale {
            if let Some(entry) = rows.remove(&id) {
                self.list.remove(&entry.list_box_row);
            }
        }

        for device in &status.devices {
            let sublabel = if device.connected {
                Some("Connected")
            } else if device.paired {
                Some("Paired")
            } else {
                None
            };

            if let Some(entry) = rows.get(&device.id) {
                let lr = entry.list_row.clone();
                let sub = sublabel.map(|s| s.to_string());
                let connected = device.connected;
                glib::idle_add_local(move || {
                    lr.set_subtitle(sub.as_deref());
                    lr.set_active(connected);
                    glib::ControlFlow::Break
                });
                continue;
            }

            let list_row = ListRow::new(
                device.name.as_deref().unwrap_or("Unknown Device"),
                &device.icon,
            );
            list_row.set_subtitle(sublabel);
            list_row.set_active(device.connected);

            let list_box_row = gtk4::ListBoxRow::builder()
                .selectable(false)
                .activatable(false)
                .child(&list_row.container)
                .build();

            let pres = self.presenter.clone();
            let device_id = device.id.clone();
            let connected = device.connected;

            let gesture = gtk4::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                if connected {
                    pres.disconnect_device(device_id.clone());
                } else {
                    pres.connect_device(device_id.clone());
                }
            });
            list_row.container.add_controller(gesture);

            rows.insert(
                device.id.clone(),
                DeviceEntry {
                    list_box_row,
                    list_row,
                },
            );
            self.list.append(&rows[&device.id].list_box_row);
        }
    }
}

impl BluetoothView for BluetoothPageView {
    fn on_connect_device(&self, _f: Box<dyn Fn(String) + 'static>) {}
    fn on_disconnect_device(&self, _f: Box<dyn Fn(String) + 'static>) {}
    fn on_set_powered(&self, _f: Box<dyn Fn(bool) + 'static>) {}
    fn on_start_scan(&self, _f: Box<dyn Fn() + 'static>) {}
    fn on_stop_scan(&self, _f: Box<dyn Fn() + 'static>) {}
}

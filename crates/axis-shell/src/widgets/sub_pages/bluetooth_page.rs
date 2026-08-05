use crate::presentation::bluetooth::BluetoothPresenter;
use crate::widgets::components::list_row::ListRow;
use crate::widgets::components::popup_header::PopupHeader;
use crate::widgets::components::scan_button::ScanButton;
use axis_domain::models::bluetooth::BluetoothStatus;
use axis_presentation::View;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

struct DeviceEntry {
    list_box_row: gtk4::ListBoxRow,
    list_row: ListRow,
    is_connected: Rc<Cell<bool>>,
}

pub struct BluetoothPage {
    pub container: gtk4::Box,
}

impl BluetoothPage {
    pub fn new(presenter: Rc<BluetoothPresenter>, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);

        let scan_btn = Rc::new(ScanButton::new());
        let header = PopupHeader::new("Bluetooth");
        header.append_suffix(scan_btn.widget());
        container.append(&header.container);

        let pres_scan = presenter.clone();
        let current_scanning = Rc::new(RefCell::new(false));
        let cs_c = current_scanning.clone();
        scan_btn.connect_clicked(move || {
            let scanning = *cs_c.borrow();
            if scanning {
                pres_scan.stop_scan();
            } else {
                pres_scan.start_scan();
            }
        });

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

        let empty_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        empty_box.set_valign(gtk4::Align::Center);
        empty_box.set_vexpand(true);
        empty_box.set_visible(false);

        let empty_icon = gtk4::Image::from_icon_name("bluetooth-disabled-symbolic");
        empty_icon.set_pixel_size(32);
        empty_icon.add_css_class("qs-empty-icon");

        let empty_title = gtk4::Label::new(Some("Bluetooth Turned Off"));
        empty_title.add_css_class("qs-empty-title");

        let empty_desc = gtk4::Label::new(Some("Turn on Bluetooth to view and connect devices."));
        empty_desc.add_css_class("qs-empty-desc");
        empty_desc.set_wrap(true);
        empty_desc.set_justify(gtk4::Justification::Center);

        empty_box.append(&empty_icon);
        empty_box.append(&empty_title);
        empty_box.append(&empty_desc);
        container.append(&empty_box);

        let on_back = Rc::new(on_back);
        header.connect_back(move || {
            on_back();
        });

        let rows: Rc<RefCell<HashMap<String, DeviceEntry>>> = Rc::new(RefCell::new(HashMap::new()));
        let rows_c = rows.clone();
        let list_c = list.clone();
        let scan_btn_c = scan_btn.clone();
        let scrolled_c = scrolled.clone();
        let empty_box_c = empty_box.clone();
        let empty_icon_c = empty_icon.clone();
        let empty_title_c = empty_title.clone();
        let empty_desc_c = empty_desc.clone();
        let presenter_c = presenter.clone();

        let view = Box::new(BluetoothPageView {
            rows: rows_c,
            list: list_c,
            scrolled: scrolled_c,
            empty_box: empty_box_c,
            empty_icon: empty_icon_c,
            empty_title: empty_title_c,
            empty_desc: empty_desc_c,
            scan_btn: scan_btn_c,
            current_scanning,
            presenter: presenter_c,
        });
        presenter.add_view(view);

        Self { container }
    }
}

struct BluetoothPageView {
    rows: Rc<RefCell<HashMap<String, DeviceEntry>>>,
    list: gtk4::ListBox,
    scrolled: gtk4::ScrolledWindow,
    empty_box: gtk4::Box,
    empty_icon: gtk4::Image,
    empty_title: gtk4::Label,
    empty_desc: gtk4::Label,
    scan_btn: Rc<ScanButton>,
    current_scanning: Rc<RefCell<bool>>,
    presenter: Rc<BluetoothPresenter>,
}

impl View<BluetoothStatus> for BluetoothPageView {
    fn render(&self, status: &BluetoothStatus) {
        *self.current_scanning.borrow_mut() = status.is_scanning;
        self.scan_btn.set_scanning(status.is_scanning);

        if !status.powered {
            self.scan_btn.widget().set_visible(false);
            self.scrolled.set_visible(false);
            self.empty_icon.set_icon_name(Some("bluetooth-disabled-symbolic"));
            self.empty_title.set_text("Bluetooth Turned Off");
            self.empty_desc.set_text("Turn on Bluetooth to view and connect devices.");
            self.empty_box.set_visible(true);
            return;
        }

        if status.devices.is_empty() {
            self.scan_btn.widget().set_visible(true);
            self.scrolled.set_visible(false);
            self.empty_icon.set_icon_name(Some("bluetooth-symbolic"));
            self.empty_title.set_text("No Devices Found");
            self.empty_desc.set_text("Make sure your device is powered on and in pairing mode.");
            self.empty_box.set_visible(true);
            return;
        }

        self.scan_btn.widget().set_visible(true);
        self.empty_box.set_visible(false);
        self.scrolled.set_visible(true);

        let mut rows = self.rows.borrow_mut();

        let ids: HashSet<String> = status.devices.iter().map(|d| d.id.clone()).collect();
        crate::utils::reconcile::reconcile(&mut rows, &ids, |_, entry| {
            self.list.remove(&entry.list_box_row);
        });

        for device in &status.devices {
            let sublabel_str = match (device.connected, device.paired, device.battery_percentage) {
                (true, _, Some(pct)) => format!("Connected • {pct}%"),
                (true, _, None) => "Connected".to_string(),
                (false, true, Some(pct)) => format!("Paired • {pct}%"),
                (false, true, None) => "Paired".to_string(),
                (false, false, Some(pct)) => format!("{pct}%"),
                (false, false, None) => String::new(),
            };
            let sublabel = if sublabel_str.is_empty() {
                None
            } else {
                Some(sublabel_str.as_str())
            };

            if let Some(entry) = rows.get(&device.id) {
                entry.list_row.set_subtitle(sublabel);
                entry.list_row.set_active(device.connected);
                entry.is_connected.set(device.connected);
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

            let is_connected = Rc::new(Cell::new(device.connected));
            let pres = self.presenter.clone();
            let device_id = device.id.clone();
            let is_conn = is_connected.clone();

            let gesture = gtk4::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                if is_conn.get() {
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
                    is_connected,
                },
            );
            self.list.append(&rows[&device.id].list_box_row);
        }
    }
}

use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::cell::RefCell;
use axis_domain::models::bluetooth::BluetoothStatus;
use crate::presentation::bluetooth::{BluetoothView, BluetoothPresenter};
use axis_presentation::View;

pub struct BluetoothPage {
    root: adw::ToolbarView,
    power_switch: adw::SwitchRow,
    device_list: gtk4::ListBox,
    
    toggle_callback: Rc<RefCell<Option<Box<dyn Fn(bool) + 'static>>>>,
    scan_callback: Rc<RefCell<Option<Box<dyn Fn(bool) + 'static>>>>,
    connect_callback: Rc<RefCell<Option<Box<dyn Fn(String) + 'static>>>>,
    disconnect_callback: Rc<RefCell<Option<Box<dyn Fn(String) + 'static>>>>,
}

impl BluetoothPage {
    pub fn new(_presenter: Rc<BluetoothPresenter>) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let preferences_page = adw::PreferencesPage::builder()
            .title("Bluetooth")
            .icon_name("bluetooth-active-symbolic")
            .build();
        toolbar_view.set_content(Some(&preferences_page));

        let power_group = adw::PreferencesGroup::builder()
            .title("Bluetooth")
            .build();
        preferences_page.add(&power_group);

        let power_switch = adw::SwitchRow::builder()
            .title("Bluetooth Enabled")
            .build();
        power_group.add(&power_switch);

        let device_group = adw::PreferencesGroup::builder()
            .title("Devices")
            .build();
        preferences_page.add(&device_group);

        let device_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        device_group.add(&device_list);

        let page = Rc::new(Self {
            root: toolbar_view,
            power_switch,
            device_list,
            toggle_callback: Rc::new(RefCell::new(None)),
            scan_callback: Rc::new(RefCell::new(None)),
            connect_callback: Rc::new(RefCell::new(None)),
            disconnect_callback: Rc::new(RefCell::new(None)),
        });

        let cb = page.toggle_callback.clone();
        page.power_switch.connect_active_notify(move |row| {
            if let Some(f) = cb.borrow().as_ref() {
                f(row.is_active());
            }
        });

        page
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }
}

impl View<BluetoothStatus> for BluetoothPage {
    fn render(&self, status: &BluetoothStatus) {
        self.power_switch.set_active(status.powered);
        
        while let Some(child) = self.device_list.first_child() {
            self.device_list.remove(&child);
        }

        if !status.powered {
            let row = adw::ActionRow::builder()
                .title("Bluetooth is disabled")
                .sensitive(false)
                .build();
            self.device_list.append(&row);
            return;
        }

        if status.devices.is_empty() {
            let row = adw::ActionRow::builder()
                .title(if status.is_scanning { "Scanning..." } else { "No devices found" })
                .sensitive(false)
                .build();
            self.device_list.append(&row);
        }

        for dev in &status.devices {
            let row = adw::ActionRow::builder()
                .title(dev.name.as_deref().unwrap_or("Unknown Device"))
                .subtitle(&dev.id)
                .build();
            
            row.add_prefix(&gtk4::Image::from_icon_name(&dev.icon));

            let connect_btn = gtk4::Button::builder()
                .label(if dev.connected { "Disconnect" } else { "Connect" })
                .valign(gtk4::Align::Center)
                .build();
            
            if dev.connected {
                connect_btn.add_css_class("destructive-action");
            } else {
                connect_btn.add_css_class("suggested-action");
            }

            let id = dev.id.clone();
            let is_connected = dev.connected;
            let cb_c = self.connect_callback.clone();
            let cb_d = self.disconnect_callback.clone();
            
            connect_btn.connect_clicked(move |_| {
                if is_connected {
                    if let Some(f) = cb_d.borrow().as_ref() { f(id.clone()); }
                } else {
                    if let Some(f) = cb_c.borrow().as_ref() { f(id.clone()); }
                }
            });

            row.add_suffix(&connect_btn);
            self.device_list.append(&row);
        }
    }
}

impl BluetoothView for BluetoothPage {
    fn on_toggle_power(&self, f: Box<dyn Fn(bool) + 'static>) {
        *self.toggle_callback.borrow_mut() = Some(f);
    }
    fn on_scan_toggled(&self, f: Box<dyn Fn(bool) + 'static>) {
        *self.scan_callback.borrow_mut() = Some(f);
    }
    fn on_device_connect(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.connect_callback.borrow_mut() = Some(f);
    }
    fn on_device_disconnect(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.disconnect_callback.borrow_mut() = Some(f);
    }
}

use crate::presentation::bluetooth::{BluetoothPresenter, BluetoothView};
use crate::widgets::callback::FnCell;
use crate::widgets::scan_button::ScanButton;
use axis_domain::models::bluetooth::BluetoothStatus;
use axis_presentation::View;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct BluetoothPage {
    root: adw::ToolbarView,
    power_switch: adw::SwitchRow,
    paired_group: adw::PreferencesGroup,
    paired_list: gtk4::ListBox,
    available_group: adw::PreferencesGroup,
    available_list: gtk4::ListBox,
    status_group: adw::PreferencesGroup,
    _status_page: adw::StatusPage,
    scan_button: Rc<ScanButton>,
    is_scanning_state: Rc<RefCell<bool>>,

    toggle_callback: FnCell<bool>,
    scan_callback: FnCell<bool>,
    connect_callback: FnCell<String>,
    disconnect_callback: FnCell<String>,
    unpair_callback: FnCell<String>,
}

impl BluetoothPage {
    pub fn new(_presenter: Rc<BluetoothPresenter>) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let scan_button = Rc::new(ScanButton::new());
        header_bar.pack_end(scan_button.widget());

        let preferences_page = adw::PreferencesPage::builder()
            .title("Bluetooth")
            .icon_name("bluetooth-active-symbolic")
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(760)
            .tightening_threshold(500)
            .child(&preferences_page)
            .build();

        toolbar_view.set_content(Some(&clamp));

        // 1. Power Switch Group
        let power_group = adw::PreferencesGroup::builder().title("Bluetooth").build();
        preferences_page.add(&power_group);

        let power_switch = adw::SwitchRow::builder().title("Bluetooth Enabled").build();
        power_group.add(&power_switch);

        // 2. Paired Devices Group
        let paired_group = adw::PreferencesGroup::builder()
            .title("Paired Devices")
            .build();
        preferences_page.add(&paired_group);

        let paired_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        paired_group.add(&paired_list);

        // 3. Available Devices Group
        let available_group = adw::PreferencesGroup::builder()
            .title("Available Devices")
            .build();
        preferences_page.add(&available_group);

        let available_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        available_group.add(&available_list);

        // 4. Empty State Status Page
        let status_group = adw::PreferencesGroup::builder().build();
        let status_page = adw::StatusPage::builder()
            .icon_name("bluetooth-disabled-symbolic")
            .title("Bluetooth is Disabled")
            .description("Turn on Bluetooth to connect to wireless devices.")
            .build();

        let enable_btn = gtk4::Button::builder()
            .label("Turn On Bluetooth")
            .css_classes(vec!["suggested-action".to_string(), "pill".to_string()])
            .halign(gtk4::Align::Center)
            .margin_top(12)
            .build();
        status_page.set_child(Some(&enable_btn));
        status_group.add(&status_page);
        status_group.set_visible(false);
        preferences_page.add(&status_group);

        let is_scanning_state = Rc::new(RefCell::new(false));

        let page = Rc::new(Self {
            root: toolbar_view,
            power_switch,
            paired_group,
            paired_list,
            available_group,
            available_list,
            status_group,
            _status_page: status_page,
            scan_button,
            is_scanning_state,
            toggle_callback: Rc::new(RefCell::new(None)),
            scan_callback: Rc::new(RefCell::new(None)),
            connect_callback: Rc::new(RefCell::new(None)),
            disconnect_callback: Rc::new(RefCell::new(None)),
            unpair_callback: Rc::new(RefCell::new(None)),
        });

        // Event Connections
        let cb_toggle = page.toggle_callback.clone();
        page.power_switch.connect_active_notify(move |row| {
            if let Some(f) = cb_toggle.borrow().as_ref() {
                f(row.is_active());
            }
        });

        let cb_enable = page.toggle_callback.clone();
        enable_btn.connect_clicked(move |_| {
            if let Some(f) = cb_enable.borrow().as_ref() {
                f(true);
            }
        });

        let cb_scan = page.scan_callback.clone();
        let scan_state_c = page.is_scanning_state.clone();
        page.scan_button.connect_clicked(move || {
            let next_state = !*scan_state_c.borrow();
            if let Some(f) = cb_scan.borrow().as_ref() {
                f(next_state);
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
        *self.is_scanning_state.borrow_mut() = status.is_scanning;
        self.scan_button.set_scanning(status.is_scanning);

        // Clear existing lists
        while let Some(child) = self.paired_list.first_child() {
            self.paired_list.remove(&child);
        }
        while let Some(child) = self.available_list.first_child() {
            self.available_list.remove(&child);
        }

        // Disabled state
        if !status.powered {
            self.paired_group.set_visible(false);
            self.available_group.set_visible(false);
            self.status_group.set_visible(true);
            return;
        }

        self.status_group.set_visible(false);

        let (paired_devs, available_devs): (Vec<_>, Vec<_>) =
            status.devices.iter().partition(|d| d.paired);

        // 1. Paired Devices Group
        if paired_devs.is_empty() {
            self.paired_group.set_visible(false);
        } else {
            self.paired_group.set_visible(true);
            for dev in paired_devs {
                let sublabel = if dev.connected { "Connected" } else { "Paired" };

                let row = adw::ActionRow::builder()
                    .title(dev.name.as_deref().unwrap_or("Unknown Device"))
                    .subtitle(sublabel)
                    .build();

                row.add_prefix(&gtk4::Image::from_icon_name(&dev.icon));

                let connect_btn = gtk4::Button::builder()
                    .label(if dev.connected {
                        "Disconnect"
                    } else {
                        "Connect"
                    })
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
                        if let Some(f) = cb_d.borrow().as_ref() {
                            f(id.clone());
                        }
                    } else if let Some(f) = cb_c.borrow().as_ref() {
                        f(id.clone());
                    }
                });

                row.add_suffix(&connect_btn);

                if !dev.connected {
                    let forget_btn = gtk4::Button::builder()
                        .label("Forget")
                        .valign(gtk4::Align::Center)
                        .css_classes(vec!["destructive-action".to_string()])
                        .build();

                    let id_forget = dev.id.clone();
                    let cb_u = self.unpair_callback.clone();
                    forget_btn.connect_clicked(move |_| {
                        if let Some(f) = cb_u.borrow().as_ref() {
                            f(id_forget.clone());
                        }
                    });

                    row.add_suffix(&forget_btn);
                }

                self.paired_list.append(&row);
            }
        }

        // 2. Available Devices Group
        self.available_group.set_visible(true);

        if available_devs.is_empty() {
            let row = adw::ActionRow::builder()
                .title(if status.is_scanning {
                    "Scanning for devices..."
                } else {
                    "No available devices found"
                })
                .sensitive(false)
                .build();
            self.available_list.append(&row);
        } else {
            for dev in available_devs {
                let row = adw::ActionRow::builder()
                    .title(dev.name.as_deref().unwrap_or("Discovered Device"))
                    .subtitle("Discovered Device")
                    .activatable(true)
                    .build();

                row.add_prefix(&gtk4::Image::from_icon_name(&dev.icon));

                let pair_btn = gtk4::Button::builder()
                    .label("Pair")
                    .valign(gtk4::Align::Center)
                    .css_classes(vec!["suggested-action".to_string()])
                    .build();

                let id = dev.id.clone();
                let cb_c = self.connect_callback.clone();
                pair_btn.connect_clicked(move |_| {
                    if let Some(f) = cb_c.borrow().as_ref() {
                        f(id.clone());
                    }
                });

                row.add_suffix(&pair_btn);

                let id_row = dev.id.clone();
                let cb_row = self.connect_callback.clone();
                row.connect_activated(move |_| {
                    if let Some(f) = cb_row.borrow().as_ref() {
                        f(id_row.clone());
                    }
                });

                self.available_list.append(&row);
            }
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
    fn on_device_unpair(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.unpair_callback.borrow_mut() = Some(f);
    }
}

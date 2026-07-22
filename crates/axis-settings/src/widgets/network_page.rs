use crate::presentation::network::{NetworkPresenter, NetworkView};
use crate::widgets::callback::{FnCell, FnCell0};
use crate::widgets::scan_button::ScanButton;
use axis_domain::models::network::NetworkStatus;
use axis_presentation::View;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type ConnectFnCell = Rc<RefCell<Option<Box<dyn Fn(String, String) + 'static>>>>;

pub struct NetworkPage {
    root: adw::ToolbarView,
    wifi_switch: adw::SwitchRow,
    connected_group: adw::PreferencesGroup,
    connected_list: gtk4::ListBox,
    ap_group: adw::PreferencesGroup,
    ap_list: gtk4::ListBox,
    status_group: adw::PreferencesGroup,
    status_page: adw::StatusPage,
    scan_button: Rc<ScanButton>,

    toggle_callback: FnCell<bool>,
    scan_callback: FnCell0,
    connect_callback: ConnectFnCell,
    disconnect_callback: FnCell0,
}

impl NetworkPage {
    pub fn new(_presenter: Rc<NetworkPresenter>) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let scan_button = Rc::new(ScanButton::new());
        header_bar.pack_end(scan_button.widget());

        let preferences_page = adw::PreferencesPage::builder()
            .title("Network")
            .icon_name("network-wireless-symbolic")
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(760)
            .tightening_threshold(500)
            .child(&preferences_page)
            .build();

        toolbar_view.set_content(Some(&clamp));

        // 1. Wi-Fi Switch Group
        let wifi_group = adw::PreferencesGroup::builder().title("Wi-Fi").build();
        preferences_page.add(&wifi_group);

        let wifi_switch = adw::SwitchRow::builder().title("Wi-Fi Enabled").build();
        wifi_group.add(&wifi_switch);

        // 2. Connected Network Group
        let connected_group = adw::PreferencesGroup::builder()
            .title("Connected Network")
            .build();
        preferences_page.add(&connected_group);

        let connected_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        connected_group.add(&connected_list);

        // 3. Available Networks Group
        let ap_group = adw::PreferencesGroup::builder()
            .title("Available Networks")
            .build();
        preferences_page.add(&ap_group);

        let ap_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        ap_group.add(&ap_list);

        // 4. Empty State / Status Page (when Wi-Fi is disabled)
        let status_group = adw::PreferencesGroup::builder().build();
        let status_page = adw::StatusPage::builder()
            .icon_name("network-wireless-disabled-symbolic")
            .title("Wi-Fi is Disabled")
            .description("Turn on Wi-Fi to connect to wireless networks.")
            .build();

        let enable_btn = gtk4::Button::builder()
            .label("Turn On Wi-Fi")
            .css_classes(vec!["suggested-action".to_string(), "pill".to_string()])
            .halign(gtk4::Align::Center)
            .margin_top(12)
            .build();
        status_page.set_child(Some(&enable_btn));
        status_group.add(&status_page);
        status_group.set_visible(false);
        preferences_page.add(&status_group);

        let page = Rc::new(Self {
            root: toolbar_view,
            wifi_switch,
            connected_group,
            connected_list,
            ap_group,
            ap_list,
            status_group,
            status_page,
            scan_button,
            toggle_callback: Rc::new(RefCell::new(None)),
            scan_callback: Rc::new(RefCell::new(None)),
            connect_callback: Rc::new(RefCell::new(None)),
            disconnect_callback: Rc::new(RefCell::new(None)),
        });

        // Event Listeners
        let cb_toggle = page.toggle_callback.clone();
        page.wifi_switch.connect_active_notify(move |row| {
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
        page.scan_button.connect_clicked(move || {
            if let Some(f) = cb_scan.borrow().as_ref() {
                f();
            }
        });

        page
    }

    fn show_password_dialog(&self, ssid: String) {
        let window = self.root.root().and_downcast::<gtk4::Window>().unwrap();
        let dialog = adw::MessageDialog::builder()
            .heading("Connect to Network")
            .body(format!("Enter password for {}", ssid))
            .transient_for(&window)
            .modal(true)
            .build();

        let entry = gtk4::PasswordEntry::builder()
            .placeholder_text("Password")
            .margin_top(12)
            .show_peek_icon(true)
            .build();

        dialog.set_extra_child(Some(&entry));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("connect", "Connect");
        dialog.set_response_appearance("connect", adw::ResponseAppearance::Suggested);

        let dialog_c = dialog.clone();
        entry.connect_activate(move |_| {
            dialog_c.response("connect");
        });

        let cb = self.connect_callback.clone();
        let ssid_c = ssid.clone();
        dialog.connect_response(None, move |d, response| {
            if response == "connect" {
                let password = entry.text().to_string();
                if let Some(f) = cb.borrow().as_ref() {
                    f(ssid_c.clone(), password);
                }
            }
            d.destroy();
        });

        dialog.present();
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }
}

impl View<NetworkStatus> for NetworkPage {
    fn render(&self, status: &NetworkStatus) {
        self.wifi_switch.set_active(status.is_wifi_enabled);
        self.scan_button.set_scanning(status.is_scanning);

        // Clear existing lists
        while let Some(child) = self.connected_list.first_child() {
            self.connected_list.remove(&child);
        }
        while let Some(child) = self.ap_list.first_child() {
            self.ap_list.remove(&child);
        }

        // Disabled State (Empty State)
        if !status.is_wifi_enabled {
            self.connected_group.set_visible(false);
            self.ap_group.set_visible(false);
            self.status_group.set_visible(true);
            return;
        }

        self.status_group.set_visible(false);

        let (active_aps, other_aps): (Vec<_>, Vec<_>) = status
            .access_points
            .iter()
            .partition(|ap| ap.is_active);

        // 1. Render Connected Network
        if let Some(active_ap) = active_aps.first() {
            self.connected_group.set_visible(true);

            let row = adw::ActionRow::builder()
                .title(&active_ap.ssid)
                .subtitle(format!("Connected · Signal {}%", active_ap.strength))
                .build();

            let strength_icon = match active_ap.strength {
                0..=20 => "network-wireless-signal-weak-symbolic",
                21..=40 => "network-wireless-signal-ok-symbolic",
                41..=60 => "network-wireless-signal-good-symbolic",
                _ => "network-wireless-signal-excellent-symbolic",
            };
            row.add_prefix(&gtk4::Image::from_icon_name(strength_icon));

            let disconnect_btn = gtk4::Button::builder()
                .label("Disconnect")
                .valign(gtk4::Align::Center)
                .css_classes(vec!["destructive-action".to_string()])
                .build();

            let cb = self.disconnect_callback.clone();
            disconnect_btn.connect_clicked(move |_| {
                if let Some(f) = cb.borrow().as_ref() {
                    f();
                }
            });
            row.add_suffix(&disconnect_btn);

            self.connected_list.append(&row);
        } else {
            self.connected_group.set_visible(false);
        }

        // 2. Render Available Networks
        self.ap_group.set_visible(true);

        if other_aps.is_empty() {
            let row = adw::ActionRow::builder()
                .title(if status.is_scanning {
                    "Scanning for networks..."
                } else {
                    "No available networks found"
                })
                .sensitive(false)
                .build();
            self.ap_list.append(&row);
            return;
        }

        for ap in other_aps {
            let row = adw::ActionRow::builder()
                .title(&ap.ssid)
                .activatable(true)
                .build();

            let strength_icon = match ap.strength {
                0..=20 => "network-wireless-signal-weak-symbolic",
                21..=40 => "network-wireless-signal-ok-symbolic",
                41..=60 => "network-wireless-signal-good-symbolic",
                _ => "network-wireless-signal-excellent-symbolic",
            };
            row.add_prefix(&gtk4::Image::from_icon_name(strength_icon));

            if ap.needs_auth {
                row.add_suffix(&gtk4::Image::from_icon_name(
                    "network-wireless-encrypted-symbolic",
                ));
            }

            let ssid = ap.ssid.clone();
            let needs_auth = ap.needs_auth;
            let page = self.clone();
            row.connect_activated(move |_| {
                if needs_auth {
                    page.show_password_dialog(ssid.clone());
                } else if let Some(f) = page.connect_callback.borrow().as_ref() {
                    f(ssid.clone(), "".to_string());
                }
            });

            self.ap_list.append(&row);
        }
    }
}

impl Clone for NetworkPage {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            wifi_switch: self.wifi_switch.clone(),
            connected_group: self.connected_group.clone(),
            connected_list: self.connected_list.clone(),
            ap_group: self.ap_group.clone(),
            ap_list: self.ap_list.clone(),
            status_group: self.status_group.clone(),
            status_page: self.status_page.clone(),
            scan_button: self.scan_button.clone(),
            toggle_callback: self.toggle_callback.clone(),
            scan_callback: self.scan_callback.clone(),
            connect_callback: self.connect_callback.clone(),
            disconnect_callback: self.disconnect_callback.clone(),
        }
    }
}

impl NetworkView for NetworkPage {
    fn on_toggle_wifi(&self, f: Box<dyn Fn(bool) + 'static>) {
        *self.toggle_callback.borrow_mut() = Some(f);
    }
    fn on_scan_requested(&self, f: Box<dyn Fn() + 'static>) {
        *self.scan_callback.borrow_mut() = Some(f);
    }
    fn on_connect(&self, f: Box<dyn Fn(String, String) + 'static>) {
        *self.connect_callback.borrow_mut() = Some(f);
    }
    fn on_disconnect(&self, f: Box<dyn Fn() + 'static>) {
        *self.disconnect_callback.borrow_mut() = Some(f);
    }
}

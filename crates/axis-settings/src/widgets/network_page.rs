use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::cell::RefCell;
use axis_domain::models::network::NetworkStatus;
use crate::presentation::network::{NetworkView, NetworkPresenter};
use axis_presentation::View;

pub struct NetworkPage {
    root: adw::ToolbarView,
    wifi_switch: adw::SwitchRow,
    ap_list: gtk4::ListBox,
    
    toggle_callback: Rc<RefCell<Option<Box<dyn Fn(bool) + 'static>>>>,
    scan_callback: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    connect_callback: Rc<RefCell<Option<Box<dyn Fn(String, String) + 'static>>>>,
    disconnect_callback: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
}

impl NetworkPage {
    pub fn new(_presenter: Rc<NetworkPresenter>) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let preferences_page = adw::PreferencesPage::builder()
            .title("Network")
            .icon_name("network-wireless-symbolic")
            .build();
        toolbar_view.set_content(Some(&preferences_page));

        let wifi_group = adw::PreferencesGroup::builder()
            .title("Wi-Fi")
            .build();
        preferences_page.add(&wifi_group);

        let wifi_switch = adw::SwitchRow::builder()
            .title("Wi-Fi Enabled")
            .build();
        wifi_group.add(&wifi_switch);

        let ap_group = adw::PreferencesGroup::builder()
            .title("Available Networks")
            .build();
        preferences_page.add(&ap_group);

        let ap_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        ap_group.add(&ap_list);

        let page = Rc::new(Self {
            root: toolbar_view,
            wifi_switch,
            ap_list,
            toggle_callback: Rc::new(RefCell::new(None)),
            scan_callback: Rc::new(RefCell::new(None)),
            connect_callback: Rc::new(RefCell::new(None)),
            disconnect_callback: Rc::new(RefCell::new(None)),
        });

        let cb = page.toggle_callback.clone();
        page.wifi_switch.connect_active_notify(move |row| {
            if let Some(f) = cb.borrow().as_ref() {
                f(row.is_active());
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
            .build();
        
        dialog.set_extra_child(Some(&entry));
        
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("connect", "Connect");
        dialog.set_response_appearance("connect", adw::ResponseAppearance::Suggested);
        
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
        
        while let Some(child) = self.ap_list.first_child() {
            self.ap_list.remove(&child);
        }

        if !status.is_wifi_enabled {
            let row = adw::ActionRow::builder()
                .title("Wi-Fi is disabled")
                .sensitive(false)
                .build();
            self.ap_list.append(&row);
            return;
        }

        if status.access_points.is_empty() {
            let row = adw::ActionRow::builder()
                .title(if status.is_scanning { "Scanning..." } else { "No networks found" })
                .sensitive(false)
                .build();
            self.ap_list.append(&row);
        }

        for ap in &status.access_points {
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

            if ap.is_active {
                row.add_suffix(&gtk4::Image::from_icon_name("emblem-ok-symbolic"));
                let disconnect_btn = gtk4::Button::builder()
                    .label("Disconnect")
                    .valign(gtk4::Align::Center)
                    .css_classes(vec!["destructive-action".to_string()])
                    .build();
                
                let cb = self.disconnect_callback.clone();
                disconnect_btn.connect_clicked(move |_| {
                    if let Some(f) = cb.borrow().as_ref() { f(); }
                });
                row.add_suffix(&disconnect_btn);
            } else {
                if ap.needs_auth {
                    row.add_suffix(&gtk4::Image::from_icon_name("network-wireless-encrypted-symbolic"));
                }
                
                let ssid = ap.ssid.clone();
                let needs_auth = ap.needs_auth;
                let page = self.clone();
                row.connect_activated(move |_| {
                    if needs_auth {
                        page.show_password_dialog(ssid.clone());
                    } else {
                        if let Some(f) = page.connect_callback.borrow().as_ref() {
                            f(ssid.clone(), "".to_string());
                        }
                    }
                });
            }
            
            self.ap_list.append(&row);
        }
    }
}

impl Clone for NetworkPage {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            wifi_switch: self.wifi_switch.clone(),
            ap_list: self.ap_list.clone(),
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

use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::bluetooth_proxy::BluetoothProxy;
use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;

pub struct BluetoothPage {
    proxy: Option<Rc<BluetoothProxy>>,
}

impl BluetoothPage {
    pub fn new(proxy: Option<&Rc<BluetoothProxy>>) -> Self {
        Self {
            proxy: proxy.cloned(),
        }
    }
}

fn device_subtitle(is_connected: bool, is_paired: bool) -> Option<String> {
    if is_connected {
        Some("Verbunden".to_string())
    } else if is_paired {
        Some("Gepaart".to_string())
    } else {
        None
    }
}

impl SettingsPage for BluetoothPage {
    fn id(&self) -> &'static str { "bluetooth" }
    fn title(&self) -> &'static str { "Bluetooth" }
    fn icon(&self) -> &'static str { "bluetooth-symbolic" }

    fn build(&self, _proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        let page = libadwaita::PreferencesPage::new();

        let bp = self.proxy.clone();

        // ── Bluetooth Toggle ───────────────────────────────────────────────
        let toggle_group = libadwaita::PreferencesGroup::builder()
            .title("Bluetooth")
            .description("Geräte drahtlos verbinden")
            .build();

        let power_switch = libadwaita::SwitchRow::builder()
            .title("Bluetooth")
            .build();

        if let Some(ref b) = bp {
            let state = b.state();
            power_switch.set_active(state.enabled);

            if state.enabled && state.devices.is_empty() {
                let b_scan = b.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = b_scan.start_scan().await;
                });
            }
        }

        if let Some(ref b) = bp {
            let b_inner = b.clone();
            power_switch.connect_active_notify(move |row| {
                let b = b_inner.clone();
                let active = row.is_active();
                gtk4::glib::spawn_future_local(async move {
                    let _ = b.set_enabled(active).await;
                    b.reload();
                });
            });
        }

        let power_switch_widget: gtk4::Widget = power_switch.into();
        toggle_group.add(&power_switch_widget);
        page.add(&toggle_group);

        // ── Devices ───────────────────────────────────────────────────────
        if let Some(ref b) = bp {
            let state = b.state();

            let devices_group = libadwaita::PreferencesGroup::builder()
                .title("Geräte")
                .build();

            for device in &state.devices {
                let row = libadwaita::ActionRow::builder()
                    .title(&device.name)
                    .build();

                let icon = gtk4::Image::from_icon_name(&device.icon);
                icon.set_pixel_size(20);
                row.add_prefix(&icon);

                if let Some(label) = device_subtitle(device.is_connected, device.is_paired) {
                    row.set_subtitle(&label);
                }

                // Click handler via activatable widget (like QS)
                let click_bridge = gtk4::Button::new();
                click_bridge.set_hexpand(true);
                click_bridge.set_valign(gtk4::Align::Center);
                row.set_activatable_widget(Some(&click_bridge));

                let path = device.path.clone();
                let bp_connect = b.clone();
                let is_connected = device.is_connected;

                click_bridge.connect_clicked(move |_| {
                    let bp = bp_connect.clone();
                    let path = path.clone();
                    gtk4::glib::spawn_future_local(async move {
                        if is_connected {
                            let _ = bp.disconnect_device(&path).await;
                        } else {
                            let _ = bp.connect_device(&path).await;
                        }
                        bp.reload();
                    });
                });

                let row_widget: gtk4::Widget = row.into();
                devices_group.add(&row_widget);
            }

            if state.devices.is_empty() && state.enabled {
                let row = libadwaita::ActionRow::builder()
                    .title("Keine Geräte gefunden")
                    .build();
                let row_widget: gtk4::Widget = row.into();
                devices_group.add(&row_widget);
            }

            page.add(&devices_group);
        }

        page.into()
    }
}

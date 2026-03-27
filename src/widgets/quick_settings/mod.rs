mod audio_page;
mod bluetooth_page;
pub mod bluetooth_pair_dialog;
pub mod components;
mod kdeconnect_page;
mod main_page;
mod nightlight_page;
mod wifi_page;

use crate::app_context::AppContext;
use crate::shell::PopupExt;
use crate::widgets::base::PopupBase;
use crate::widgets::quick_settings::nightlight_page::NightlightPage;
use audio_page::AudioPage;
use bluetooth_page::BluetoothPage;
use kdeconnect_page::KdeConnectPage;
use main_page::MainPage;
use wifi_page::WifiPage;

use crate::services::bluetooth::BluetoothCmd;
use crate::services::network::NetworkCmd;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct QuickSettingsPopup {
    pub base: PopupBase,
    pub container: gtk4::Box,
    pub archive_box: gtk4::Box,
    qs_stack: gtk4::Stack,
    main_page: MainPage,
}

impl PopupExt for QuickSettingsPopup {
    fn id(&self) -> &str {
        "qs"
    }

    fn base(&self) -> &PopupBase {
        &self.base
    }

    fn on_close(&self) {
        self.qs_stack.set_visible_child_name("main");
        let _ = self.base.window; // ensure field access compiles
    }

    fn handle_escape(&self) {
        let current = self.qs_stack.visible_child_name();
        if current.as_deref() == Some("main") {
            if self.main_page.is_power_expanded() {
                self.main_page.collapse_power_menu();
            } else {
                self.close();
            }
        } else {
            self.qs_stack.set_visible_child_name("main");
        }
    }
}

impl QuickSettingsPopup {
    pub fn new(
        app: &libadwaita::Application,
        vol_icon_bar: &gtk4::Image,
        ctx: AppContext,
        on_lock: Rc<dyn Fn()>,
    ) -> Self {
        let base = PopupBase::new(app, "AXIS Quick Settings", true);

        // Stop BT scan on close
        let tx_bt_stop = ctx.bluetooth.tx.clone();
        base.window.connect_visible_notify(move |win| {
            if !win.is_visible() {
                let _ = tx_bt_stop.try_send(BluetoothCmd::StopScan);
            }
        });

        let qs_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        qs_container.add_css_class("qs-panel");
        qs_container.set_width_request(380);
        qs_container.set_valign(gtk4::Align::End);

        let qs_stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::SlideLeftRight)
            .transition_duration(250)
            .vhomogeneous(false)
            .hhomogeneous(true)
            .interpolate_size(true)
            .build();

        // --- PAGES ---
        let stack_wifi = qs_stack.clone();
        let tx_wifi = ctx.network.tx.clone();
        let open_wifi = move || {
            stack_wifi.set_visible_child_name("wifi");
            let _ = tx_wifi.try_send(NetworkCmd::ScanWifi);
        };

        let stack_bt = qs_stack.clone();
        let tx_bt = ctx.bluetooth.tx.clone();
        let open_bt = move || {
            stack_bt.set_visible_child_name("bluetooth");
            let _ = tx_bt.try_send(BluetoothCmd::Scan);
        };

        let stack_nl = qs_stack.clone();
        let open_nl = move || {
            stack_nl.set_visible_child_name("nightlight");
        };

        let stack_audio = qs_stack.clone();
        let open_audio = move || {
            stack_audio.set_visible_child_name("audio");
        };

        let stack_kc = qs_stack.clone();
        let open_kdeconnect = move || {
            stack_kc.set_visible_child_name("kdeconnect");
        };

        let main_page = MainPage::new(
            ctx.clone(),
            vol_icon_bar.clone(),
            open_wifi,
            open_bt,
            open_nl,
            open_audio,
            open_kdeconnect,
            on_lock,
        );

        let stack_back = qs_stack.clone();
        let wifi_page = WifiPage::new(
            ctx.clone(),
            move || {
                stack_back.set_visible_child_name("main");
            },
            main_page.wifi_tile.clone(),
            main_page.eth_tile.clone(),
        );

        let stack_back_bt = qs_stack.clone();
        let bluetooth_page = BluetoothPage::new(
            ctx.clone(),
            move || {
                stack_back_bt.set_visible_child_name("main");
            },
            main_page.bt_tile.clone(),
        );

        let stack_back_nl = qs_stack.clone();
        let nightlight_page = NightlightPage::new(
            ctx.clone(),
            move || {
                stack_back_nl.set_visible_child_name("main");
            },
            main_page.nl_tile.clone(),
            ctx.nightlight.tx.clone(),
        );

        let stack_back_audio = qs_stack.clone();
        let audio_page = AudioPage::new(
            ctx.clone(),
            move || {
                stack_back_audio.set_visible_child_name("main");
            },
        );

        let stack_back_kc = qs_stack.clone();
        let kdeconnect_page = KdeConnectPage::new(
            ctx.clone(),
            move || {
                stack_back_kc.set_visible_child_name("main");
            },
        );

        qs_stack.add_named(&main_page.container, Some("main"));
        qs_stack.add_named(&wifi_page.container, Some("wifi"));
        qs_stack.add_named(&bluetooth_page.container, Some("bluetooth"));
        qs_stack.add_named(&nightlight_page.container, Some("nightlight"));
        qs_stack.add_named(&audio_page.container, Some("audio"));
        qs_stack.add_named(&kdeconnect_page.container, Some("kdeconnect"));

        qs_container.append(&qs_stack);

        // Outer wrapper: Archive oben, Popup-Content unten
        let archive_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        archive_box.set_valign(gtk4::Align::End);
        archive_box.set_halign(gtk4::Align::End);
        archive_box.set_width_request(380);

        let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        outer.set_valign(gtk4::Align::End);
        outer.append(&archive_box);
        outer.append(&qs_container);
        base.set_content(&outer);

        // Custom escape handler for stack navigation (register() wires Escape to handle_escape())
        let stack_kb = qs_stack.clone();
        let main_page_kb = main_page.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                // Handle at entry level before window-level handler
                let current = stack_kb.visible_child_name();
                if current.as_deref() == Some("main") {
                    if main_page_kb.is_power_expanded() {
                        main_page_kb.collapse_power_menu();
                        return gtk4::glib::Propagation::Stop;
                    }
                } else {
                    stack_kb.set_visible_child_name("main");
                    return gtk4::glib::Propagation::Stop;
                }
            }
            gtk4::glib::Propagation::Proceed
        });
        qs_container.add_controller(key_controller);

        Self {
            base,
            container: qs_container,
            archive_box,
            qs_stack,
            main_page,
        }
    }
}

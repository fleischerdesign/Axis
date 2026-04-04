mod audio_page;
mod bluetooth_page;
pub mod bluetooth_pair_dialog;
mod continuity_page;
pub mod continuity_pair_dialog;
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
use continuity_page::ContinuityPage;
use kdeconnect_page::KdeConnectPage;
use main_page::MainPage;
use wifi_page::WifiPage;

use axis_core::services::bluetooth::BluetoothCmd;
use axis_core::services::network::NetworkCmd;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct QuickSettingsPopup {
    base: PopupBase,
    container: gtk4::Box,
    archive_box: gtk4::Box,
    qs_stack: gtk4::Stack,
    main_page: MainPage,
    bt_tx: async_channel::Sender<BluetoothCmd>,
    ct_tx: async_channel::Sender<axis_core::services::continuity::ContinuityCmd>,
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
        let _ = self.bt_tx.try_send(BluetoothCmd::StopScan);
        let _ = self.ct_tx.try_send(axis_core::services::continuity::ContinuityCmd::StopDiscovery);
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

        let bt_tx = ctx.bluetooth.tx.clone();
        let ct_tx = ctx.continuity.tx.clone();

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

        let stack_ct = qs_stack.clone();
        let tx_ct = ctx.continuity.tx.clone();
        let open_continuity = move || {
            stack_ct.set_visible_child_name("continuity");
            let _ = tx_ct.try_send(axis_core::services::continuity::ContinuityCmd::StartDiscovery);
        };

        let main_page = MainPage::new(
            ctx.clone(),
            vol_icon_bar.clone(),
            open_wifi,
            open_bt,
            open_nl,
            open_audio,
            open_kdeconnect,
            open_continuity,
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
            main_page.bluetooth_tile.clone(),
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

        let stack_back_ct = qs_stack.clone();
        let continuity_page = ContinuityPage::new(
            ctx.clone(),
            move || {
                stack_back_ct.set_visible_child_name("main");
            },
        );

        qs_stack.add_named(&main_page.container, Some("main"));
        qs_stack.add_named(&wifi_page.container, Some("wifi"));
        qs_stack.add_named(&bluetooth_page.container, Some("bluetooth"));
        qs_stack.add_named(&nightlight_page.container, Some("nightlight"));
        qs_stack.add_named(&audio_page.container, Some("audio"));
        qs_stack.add_named(&kdeconnect_page.container, Some("kdeconnect"));
        qs_stack.add_named(&continuity_page.container, Some("continuity"));

        qs_container.append(&qs_stack);

        // Outer wrapper: Archive above QS content
        let archive_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        archive_box.set_valign(gtk4::Align::End);
        archive_box.set_halign(gtk4::Align::End);
        archive_box.set_width_request(380);

        let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        outer.set_valign(gtk4::Align::End);
        outer.append(&archive_box);
        outer.append(&qs_container);
        base.set_content(&outer);

        Self {
            base,
            container: qs_container,
            archive_box,
            qs_stack,
            main_page,
            bt_tx,
            ct_tx,
        }
    }

    pub fn archive_container(&self) -> &gtk4::Box {
        &self.archive_box
    }
}

mod bluetooth_page;
pub mod components;
mod main_page;
mod nightlight_page;
mod wifi_page;

use crate::app_context::AppContext;
use crate::widgets::quick_settings::nightlight_page::NightlightPage;
use bluetooth_page::BluetoothPage;
use main_page::MainPage;
use wifi_page::WifiPage;
use crate::shell::ShellPopup;
use crate::widgets::base::PopupBase;

use crate::services::bluetooth::BluetoothCmd;
use crate::services::network::NetworkCmd;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct QuickSettingsPopup {
    pub base: PopupBase,
}

impl ShellPopup for QuickSettingsPopup {
    fn id(&self) -> &str { "qs" }
    fn is_open(&self) -> bool { *self.base.is_open.borrow() }

    fn close(&self) {
        self.base.close();
    }

    fn toggle(&self) {
        self.base.toggle();
    }
}

impl QuickSettingsPopup {
    pub fn new(
        app: &libadwaita::Application, 
        vol_icon_bar: &gtk4::Image, 
        ctx: AppContext,
        on_state_change: impl Fn() + 'static,
    ) -> Self {
        let base = PopupBase::new(app, "Carp Quick Settings", true);
        let on_state_change = Rc::new(on_state_change);

        // State-Change an den Controller melden
        let on_change_c = on_state_change.clone();
        base.window.connect_visible_notify(move |_| {
            on_change_c();
        });

        let qs_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        qs_container.add_css_class("qs-panel");
        qs_container.set_width_request(340);

        let qs_stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::SlideLeftRight)
            .transition_duration(250)
            .vhomogeneous(false)
            .hhomogeneous(false)
            .interpolate_size(true)
            .build();

        // --- PAGES ---
        let stack_wifi = qs_stack.clone();
        let tx_wifi = ctx.network_tx.clone();
        let open_wifi = move || {
            stack_wifi.set_visible_child_name("wifi");
            let _ = tx_wifi.send_blocking(NetworkCmd::ScanWifi);
        };

        let stack_bt = qs_stack.clone();
        let tx_bt = ctx.bluetooth_tx.clone();
        let open_bt = move || {
            stack_bt.set_visible_child_name("bluetooth");
            let _ = tx_bt.send_blocking(BluetoothCmd::Scan);
        };

        let stack_nl = qs_stack.clone();
        let open_nl = move || {
            stack_nl.set_visible_child_name("nightlight");
        };

        let main_page = MainPage::new(
            ctx.clone(),
            vol_icon_bar.clone(),
            open_wifi,
            open_bt,
            open_nl,
        );

        let stack_back = qs_stack.clone();
        let win_back = base.window.clone();
        let wifi_page = WifiPage::new(
            ctx.clone(),
            move || {
                stack_back.set_visible_child_name("main");
                win_back.set_default_size(1, 1);
            },
            main_page.wifi_tile.clone(),
            main_page.eth_tile.clone(),
        );

        let stack_back_bt = qs_stack.clone();
        let win_back_bt = base.window.clone();
        let bluetooth_page = BluetoothPage::new(
            ctx.clone(),
            move || {
                stack_back_bt.set_visible_child_name("main");
                win_back_bt.set_default_size(1, 1);
            },
            main_page.bt_tile.clone(),
        );

        let stack_back_nl = qs_stack.clone();
        let win_back_nl = base.window.clone();
        let nightlight_page = NightlightPage::new(
            ctx.clone(),
            move || {
                stack_back_nl.set_visible_child_name("main");
                win_back_nl.set_default_size(1, 1);
            },
            main_page.nl_tile.clone(),
            ctx.nightlight_tx.clone(),
        );

        qs_stack.add_named(&main_page.container, Some("main"));
        qs_stack.add_named(&wifi_page.container, Some("wifi"));
        qs_stack.add_named(&bluetooth_page.container, Some("bluetooth"));
        qs_stack.add_named(&nightlight_page.container, Some("nightlight"));
        qs_container.append(&qs_stack);
        base.set_content(&qs_container);

        Self { base }
    }
}

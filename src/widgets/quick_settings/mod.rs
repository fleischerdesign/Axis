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

use crate::services::bluetooth::BluetoothCmd;
use crate::services::network::NetworkCmd;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

pub struct QuickSettingsPopup {
    pub window: gtk4::Window,
    pub is_open: Rc<RefCell<bool>>,
    on_state_change: Rc<dyn Fn() + 'static>,
}

impl ShellPopup for QuickSettingsPopup {
    fn id(&self) -> &str { "qs" }
    fn is_open(&self) -> bool { *self.is_open.borrow() }

    fn close(&self) {
        if !self.is_open() { return; }
        *self.is_open.borrow_mut() = false;
        self.animate_close();
        (self.on_state_change)();
    }

    fn toggle(&self) {
        if self.is_open() {
            self.close();
        } else {
            *self.is_open.borrow_mut() = true;
            self.animate_open();
            (self.on_state_change)();
        }
    }
}

impl QuickSettingsPopup {
    pub fn new(
        app: &libadwaita::Application, 
        vol_icon_bar: &gtk4::Image, 
        ctx: AppContext,
        on_state_change: impl Fn() + 'static,
    ) -> Self {
        let is_open = Rc::new(RefCell::new(false));
        let on_state_change = Rc::new(on_state_change);

        let window = gtk4::Window::builder()
            .application(app)
            .title("Carp Quick Settings")
            .visible(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Bottom, 64);
        window.set_margin(Edge::Right, 10);

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
        let win_back = window.clone();
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
        let win_back_bt = window.clone();
        let bluetooth_page = BluetoothPage::new(
            ctx.clone(),
            move || {
                stack_back_bt.set_visible_child_name("main");
                win_back_bt.set_default_size(1, 1);
            },
            main_page.bt_tile.clone(),
        );

        let stack_back_nl = qs_stack.clone();
        let win_back_nl = window.clone();
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
        let qs_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::Crossfade)
            .transition_duration(250)
            .build();
        qs_revealer.set_child(Some(&qs_container));
        window.set_child(Some(&qs_revealer));

        Self { window, is_open, on_state_change }
    }

    fn animate_open(&self) {
        self.window.set_visible(true);
        if let Some(rev) = self.window.child().and_then(|c| c.downcast::<gtk4::Revealer>().ok()) {
            rev.set_reveal_child(true);
        }
    }

    fn animate_close(&self) {
        if let Some(rev) = self.window.child().and_then(|c| c.downcast::<gtk4::Revealer>().ok()) {
            rev.set_reveal_child(false);
            let win = self.window.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(280), move || {
                win.set_visible(false);
                gtk4::glib::ControlFlow::Break
            });
        }
    }
}

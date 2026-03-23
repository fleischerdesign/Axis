use crate::services::bluetooth::{self, BluetoothCmd, PairingRequest, PairingType};
use async_channel::Sender;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::Cell;
use std::rc::Rc;

pub struct BluetoothPairingDialog {
    window: gtk4::Window,
    device_label: gtk4::Label,
    prompt_label: gtk4::Label,
    passkey_label: gtk4::Label,
    is_showing: Rc<Cell<bool>>,
}

impl BluetoothPairingDialog {
    pub fn new(app: &libadwaita::Application, tx: Sender<BluetoothCmd>) -> Self {
        let window = gtk4::Window::builder()
            .application(app)
            .title("Bluetooth Pairing")
            .visible(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);

        // Dim background
        let overlay = gtk4::Overlay::new();
        let dim = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        dim.set_hexpand(true);
        dim.set_vexpand(true);
        dim.add_css_class("bt-pair-dim");
        overlay.set_child(Some(&dim));

        // Center card
        let card = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        card.add_css_class("bt-pair-card");
        card.set_halign(gtk4::Align::Center);
        card.set_valign(gtk4::Align::Center);

        let icon = gtk4::Image::from_icon_name("bluetooth-active-symbolic");
        icon.set_pixel_size(48);
        card.append(&icon);

        let device_label = gtk4::Label::new(None);
        device_label.add_css_class("bt-pair-device");
        card.append(&device_label);

        let prompt_label = gtk4::Label::new(None);
        prompt_label.add_css_class("bt-pair-prompt");
        prompt_label.set_wrap(true);
        prompt_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        prompt_label.set_max_width_chars(30);
        card.append(&prompt_label);

        let passkey_label = gtk4::Label::new(None);
        passkey_label.add_css_class("bt-pair-passkey");
        passkey_label.set_visible(false);
        card.append(&passkey_label);

        let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        buttons.set_halign(gtk4::Align::Center);
        buttons.set_margin_top(8);

        let reject_btn = gtk4::Button::with_label("Ablehnen");
        reject_btn.add_css_class("bt-pair-reject");
        reject_btn.set_hexpand(true);

        let accept_btn = gtk4::Button::with_label("Bestätigen");
        accept_btn.add_css_class("bt-pair-accept");
        accept_btn.set_hexpand(true);

        buttons.append(&reject_btn);
        buttons.append(&accept_btn);
        card.append(&buttons);

        overlay.add_overlay(&card);
        window.set_child(Some(&overlay));

        // Wire buttons
        accept_btn.connect_clicked({
            let tx = tx.clone();
            move |_| {
                let _ = tx.try_send(BluetoothCmd::PairAccept);
            }
        });

        reject_btn.connect_clicked({
            let tx = tx.clone();
            move |_| {
                let _ = tx.try_send(BluetoothCmd::PairReject);
            }
        });

        // Escape to reject
        let tx_esc = tx;
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                let _ = tx_esc.try_send(BluetoothCmd::PairReject);
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        window.add_controller(key_controller);

        Self {
            window,
            device_label,
            prompt_label,
            passkey_label,
            is_showing: Rc::new(Cell::new(false)),
        }
    }

    fn show(&self, req: &PairingRequest) {
        if self.is_showing.get() {
            return;
        }
        log::info!("[bluetooth] Pairing dialog SHOW: {}", req.device_name);
        self.device_label.set_text(&req.device_name);
        self.prompt_label.set_text(match req.pairing_type {
            PairingType::Confirmation => {
                "Bestätigen Sie, dass der PIN auf dem Gerät übereinstimmt."
            }
            PairingType::PinCode => "Geben Sie den PIN-Code ein, der am Gerät angezeigt wird.",
            PairingType::Passkey => "Geben Sie den Passkey ein.",
            PairingType::Authorization => "Möchten Sie die Kopplung erlauben?",
        });
        if let Some(ref pk) = req.passkey {
            self.passkey_label.set_text(pk);
            self.passkey_label.set_visible(true);
        } else {
            self.passkey_label.set_visible(false);
        }
        self.is_showing.set(true);
        self.window.set_visible(true);
    }

    fn hide(&self) {
        if !self.is_showing.get() {
            return;
        }
        log::info!("[bluetooth] Pairing dialog HIDE");
        self.is_showing.set(false);
        self.window.set_visible(false);
    }
}

pub fn spawn_pairing_dialog(app: &libadwaita::Application, tx: Sender<BluetoothCmd>) {
    let dialog = BluetoothPairingDialog::new(app, tx);
    log::info!("[bluetooth] Pairing dialog timer started");

    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
        let req = bluetooth::get_pairing_request();
        match req {
            Some(ref r) => dialog.show(r),
            None => dialog.hide(),
        }
        gtk4::glib::ControlFlow::Continue
    });
}

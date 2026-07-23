use crate::widgets::callback::{FnCell, FnCell0};
use axis_domain::models::continuity::{ContinuityStatus, PeerConfig};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type ConfigFnCell = Rc<RefCell<Option<Box<dyn Fn(String, PeerConfig) + 'static>>>>;

pub struct PeerDetailPage {
    root: adw::Clamp,
    peer_id: String,
    auto_connect_switch: adw::SwitchRow,
    clipboard_switch: adw::SwitchRow,
    audio_switch: adw::SwitchRow,
    audio_direction_row: adw::ComboRow,
    audio_source_row: adw::ComboRow,
    drag_drop_switch: adw::SwitchRow,
    danger_group: adw::PreferencesGroup,
    disconnect_btn: gtk4::Button,
    unpair_btn: gtk4::Button,

    update_silent: Rc<RefCell<bool>>,
    last_config: Rc<RefCell<Option<PeerConfig>>>,
    disconnect_cb: FnCell0,
    unpair_cb: FnCell<String>,
    config_cb: ConfigFnCell,
}

impl PeerDetailPage {
    pub fn new(peer_id: String, peer_name: String) -> Rc<Self> {
        let page = adw::PreferencesPage::builder()
            .title(&peer_name)
            .icon_name("computer-symbolic")
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(760)
            .tightening_threshold(500)
            .child(&page)
            .build();

        let caps_group = adw::PreferencesGroup::builder()
            .title("Capabilities &amp; Automation")
            .description("Configure sharing permissions and automatic connection for this device")
            .build();
        page.add(&caps_group);

        let auto_connect_switch = adw::SwitchRow::builder()
            .title("Auto-Connect")
            .subtitle("Automatically connect when this trusted peer is in range")
            .build();
        caps_group.add(&auto_connect_switch);

        let clipboard_switch = adw::SwitchRow::builder()
            .title("Synchronize Clipboard")
            .subtitle("Share clipboard between both devices")
            .build();
        caps_group.add(&clipboard_switch);

        let audio_switch = adw::SwitchRow::builder()
            .title("Audio Sharing")
            .subtitle("Stream audio playback to this device")
            .build();
        caps_group.add(&audio_switch);

        let audio_dir_model = gtk4::StringList::new(&[
            "Aus",
            "Dieser PC sendet →",
            "← Empfangen",
            "⇄ Beidseitig (Duplex)",
        ]);
        let audio_direction_row = adw::ComboRow::builder()
            .title("Audio-Richtung (Direction)")
            .subtitle("Steuert wer Ton sendet oder empfängt")
            .model(&audio_dir_model)
            .build();
        caps_group.add(&audio_direction_row);

        let audio_source_model = gtk4::StringList::new(&[
            "System-Sound (Spotify, Browser, Media)",
            "Standard Mikrofon",
        ]);
        let audio_source_row = adw::ComboRow::builder()
            .title("Aufnahme-Quelle (Capture Source)")
            .subtitle("Wähle Medienton-Monitor oder Mikrofon zum Senden")
            .model(&audio_source_model)
            .build();
        caps_group.add(&audio_source_row);

        let drag_drop_switch = adw::SwitchRow::builder()
            .title("Drag &amp; Drop")
            .subtitle("Transfer files via drag &amp; drop")
            .build();
        caps_group.add(&drag_drop_switch);

        let danger_group = adw::PreferencesGroup::builder()
            .title("Device Actions")
            .build();
        danger_group.set_visible(false);
        page.add(&danger_group);

        let disconnect_btn = gtk4::Button::builder()
            .label("Disconnect")
            .css_classes(vec!["destructive-action".to_string(), "pill".to_string()])
            .visible(false)
            .build();

        let unpair_btn = gtk4::Button::builder()
            .label("Unpair Device")
            .css_classes(vec!["destructive-action".to_string(), "pill".to_string()])
            .visible(false)
            .build();

        let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        btn_box.set_halign(gtk4::Align::Center);
        btn_box.set_margin_top(8);
        btn_box.append(&disconnect_btn);
        btn_box.append(&unpair_btn);
        danger_group.add(&btn_box);

        let page = Rc::new(Self {
            root: clamp,
            peer_id,
            auto_connect_switch,
            clipboard_switch,
            audio_switch,
            audio_direction_row,
            audio_source_row,
            drag_drop_switch,
            danger_group,
            disconnect_btn,
            unpair_btn,
            update_silent: Rc::new(RefCell::new(false)),
            last_config: Rc::new(RefCell::new(None)),
            disconnect_cb: Rc::new(RefCell::new(None)),
            unpair_cb: Rc::new(RefCell::new(None)),
            config_cb: Rc::new(RefCell::new(None)),
        });

        Self::wire_notifies(&page);

        page
    }

    fn wire_notifies(page: &Rc<Self>) {
        let p = page.clone();
        page.auto_connect_switch.connect_active_notify(move |row| {
            if *p.update_silent.borrow() {
                return;
            }
            let current = p.last_config.borrow().clone().unwrap_or_default();
            let config = PeerConfig {
                auto_connect: row.is_active(),
                ..current
            };
            if let Some(f) = p.config_cb.borrow().as_ref() {
                f(p.peer_id.clone(), config);
            }
        });

        let p = page.clone();
        page.clipboard_switch.connect_active_notify(move |row| {
            if *p.update_silent.borrow() {
                return;
            }
            let current = p.last_config.borrow().clone().unwrap_or_default();
            let config = PeerConfig {
                clipboard: row.is_active(),
                ..current
            };
            if let Some(f) = p.config_cb.borrow().as_ref() {
                f(p.peer_id.clone(), config);
            }
        });

        let p = page.clone();
        page.audio_switch.connect_active_notify(move |row| {
            if *p.update_silent.borrow() {
                return;
            }
            let current = p.last_config.borrow().clone().unwrap_or_default();
            let config = PeerConfig {
                audio: row.is_active(),
                ..current
            };
            if let Some(f) = p.config_cb.borrow().as_ref() {
                f(p.peer_id.clone(), config);
            }
        });

        let p = page.clone();
        page.audio_direction_row
            .connect_selected_notify(move |row| {
                if *p.update_silent.borrow() {
                    return;
                }
                let current = p.last_config.borrow().clone().unwrap_or_default();
                let dir = match row.selected() {
                    1 => axis_domain::models::continuity::AudioStreamDirection::SendToPeer,
                    2 => axis_domain::models::continuity::AudioStreamDirection::ReceiveFromPeer,
                    3 => axis_domain::models::continuity::AudioStreamDirection::BiDirectional,
                    _ => axis_domain::models::continuity::AudioStreamDirection::Off,
                };
                let config = PeerConfig {
                    audio_direction: dir,
                    audio: dir != axis_domain::models::continuity::AudioStreamDirection::Off,
                    ..current
                };
                if let Some(f) = p.config_cb.borrow().as_ref() {
                    f(p.peer_id.clone(), config);
                }
            });

        let p = page.clone();
        page.audio_source_row
            .connect_selected_notify(move |row| {
                if *p.update_silent.borrow() {
                    return;
                }
                let current = p.last_config.borrow().clone().unwrap_or_default();
                let capture_device = match row.selected() {
                    1 => Some("@DEFAULT_SOURCE@".to_string()),
                    _ => Some("@DEFAULT_MONITOR@".to_string()),
                };
                let config = PeerConfig {
                    capture_device,
                    ..current
                };
                if let Some(f) = p.config_cb.borrow().as_ref() {
                    f(p.peer_id.clone(), config);
                }
            });

        let p = page.clone();
        page.drag_drop_switch.connect_active_notify(move |row| {
            if *p.update_silent.borrow() {
                return;
            }
            let current = p.last_config.borrow().clone().unwrap_or_default();
            let config = PeerConfig {
                drag_drop: row.is_active(),
                ..current
            };
            if let Some(f) = p.config_cb.borrow().as_ref() {
                f(p.peer_id.clone(), config);
            }
        });

        let p = page.clone();
        page.disconnect_btn.connect_clicked(move |_| {
            if let Some(f) = p.disconnect_cb.borrow().as_ref() {
                f();
            }
        });

        let p = page.clone();
        page.unpair_btn.connect_clicked(move |_| {
            if let Some(f) = p.unpair_cb.borrow().as_ref() {
                f(p.peer_id.clone());
            }
        });
    }

    pub fn widget(&self) -> &adw::Clamp {
        &self.root
    }

    pub fn update_status(&self, status: &ContinuityStatus) {
        *self.update_silent.borrow_mut() = true;

        let found_config = status.peer_configs.get(&self.peer_id).or_else(|| {
            if let Some(p) = status.peers.iter().find(|p| {
                p.device_name == self.peer_id
                    || p.hostname == self.peer_id
                    || p.device_id == self.peer_id
            })
                && let Some(cfg) = status.peer_configs.get(&p.device_id)
            {
                return Some(cfg);
            }
            None
        });

        let is_paired = found_config.is_some() || status.peer_configs.contains_key(&self.peer_id);
        if let Some(config) = found_config {
            *self.last_config.borrow_mut() = Some(config.clone());
            self.auto_connect_switch.set_active(config.auto_connect);
            self.clipboard_switch.set_active(config.clipboard);
            self.audio_switch.set_active(config.audio);

            let dir_selected = match config.audio_direction {
                axis_domain::models::continuity::AudioStreamDirection::Off => 0,
                axis_domain::models::continuity::AudioStreamDirection::SendToPeer => 1,
                axis_domain::models::continuity::AudioStreamDirection::ReceiveFromPeer => 2,
                axis_domain::models::continuity::AudioStreamDirection::BiDirectional => 3,
            };
            self.audio_direction_row.set_selected(dir_selected);

            let selected = match config.capture_device.as_deref() {
                Some("@DEFAULT_SOURCE@") => 1,
                _ => 0,
            };
            self.audio_source_row.set_selected(selected);

            self.drag_drop_switch.set_active(config.drag_drop);
        }

        let connected = status
            .active_connection
            .as_ref()
            .is_some_and(|c| c.peer_id == self.peer_id);

        self.disconnect_btn.set_visible(connected);
        self.unpair_btn.set_visible(is_paired);
        self.danger_group.set_visible(connected || is_paired);

        *self.update_silent.borrow_mut() = false;
    }

    pub fn set_on_disconnect(&self, f: Box<dyn Fn() + 'static>) {
        *self.disconnect_cb.borrow_mut() = Some(f);
    }

    pub fn set_on_unpair(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.unpair_cb.borrow_mut() = Some(f);
    }

    pub fn set_on_config(&self, f: Box<dyn Fn(String, PeerConfig) + 'static>) {
        *self.config_cb.borrow_mut() = Some(f);
    }
}

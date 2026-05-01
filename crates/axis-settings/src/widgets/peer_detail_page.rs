use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::cell::RefCell;
use axis_domain::models::continuity::{ContinuityStatus, PeerConfig};

pub struct PeerDetailPage {
    root: adw::ToolbarView,
    peer_id: String,
    clipboard_switch: adw::SwitchRow,
    audio_switch: adw::SwitchRow,
    drag_drop_switch: adw::SwitchRow,
    disconnect_btn: gtk4::Button,
    unpair_btn: gtk4::Button,

    update_silent: Rc<RefCell<bool>>,
    last_config: Rc<RefCell<Option<PeerConfig>>>,
    disconnect_cb: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    unpair_cb: Rc<RefCell<Option<Box<dyn Fn(String) + 'static>>>>,
    config_cb: Rc<RefCell<Option<Box<dyn Fn(String, PeerConfig) + 'static>>>>,
}

impl PeerDetailPage {
    pub fn new(peer_id: String, peer_name: String) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let page = adw::PreferencesPage::builder()
            .title(&peer_name)
            .icon_name("input-mouse-symbolic")
            .build();
        toolbar_view.set_content(Some(&page));

        let caps_group = adw::PreferencesGroup::builder()
            .title("Capabilities")
            .build();
        page.add(&caps_group);

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

        let drag_drop_switch = adw::SwitchRow::builder()
            .title("Drag & Drop")
            .subtitle("Transfer files via drag & drop")
            .build();
        caps_group.add(&drag_drop_switch);

        let danger_group = adw::PreferencesGroup::builder()
            .title("")
            .build();
        page.add(&danger_group);

        let disconnect_btn = gtk4::Button::builder()
            .label("Disconnect")
            .css_classes(vec!["destructive-action".to_string()])
            .build();

        let unpair_btn = gtk4::Button::builder()
            .label("Unpair")
            .css_classes(vec!["destructive-action".to_string()])
            .build();

        let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        btn_box.append(&disconnect_btn);
        btn_box.append(&unpair_btn);
        danger_group.add(&btn_box);

        let page = Rc::new(Self {
            root: toolbar_view,
            peer_id,
            clipboard_switch,
            audio_switch,
            drag_drop_switch,
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
        page.clipboard_switch.connect_active_notify(move |row| {
            if *p.update_silent.borrow() { return; }
            if let Some(f) = p.config_cb.borrow().as_ref() {
                if let Some(ref current) = *p.last_config.borrow() {
                    let config = PeerConfig { clipboard: row.is_active(), ..current.clone() };
                    f(p.peer_id.clone(), config);
                }
            }
        });

        let p = page.clone();
        page.audio_switch.connect_active_notify(move |row| {
            if *p.update_silent.borrow() { return; }
            if let Some(f) = p.config_cb.borrow().as_ref() {
                if let Some(ref current) = *p.last_config.borrow() {
                    let config = PeerConfig { audio: row.is_active(), ..current.clone() };
                    f(p.peer_id.clone(), config);
                }
            }
        });

        let p = page.clone();
        page.drag_drop_switch.connect_active_notify(move |row| {
            if *p.update_silent.borrow() { return; }
            if let Some(f) = p.config_cb.borrow().as_ref() {
                if let Some(ref current) = *p.last_config.borrow() {
                    let config = PeerConfig { drag_drop: row.is_active(), ..current.clone() };
                    f(p.peer_id.clone(), config);
                }
            }
        });

        let p = page.clone();
        page.disconnect_btn.connect_clicked(move |_| {
            if let Some(f) = p.disconnect_cb.borrow().as_ref() { f(); }
        });

        let p = page.clone();
        page.unpair_btn.connect_clicked(move |_| {
            if let Some(f) = p.unpair_cb.borrow().as_ref() { f(p.peer_id.clone()); }
        });
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }

    #[allow(dead_code)]
    pub fn update_status(&self, status: &ContinuityStatus) {
        *self.update_silent.borrow_mut() = true;

        if let Some(config) = status.peer_configs.get(&self.peer_id) {
            *self.last_config.borrow_mut() = Some(config.clone());
            self.clipboard_switch.set_active(config.clipboard);
            self.audio_switch.set_active(config.audio);
            self.drag_drop_switch.set_active(config.drag_drop);
        }

        let connected = status.active_connection
            .as_ref()
            .is_some_and(|c| c.peer_id == self.peer_id);
        self.disconnect_btn.set_visible(connected);

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


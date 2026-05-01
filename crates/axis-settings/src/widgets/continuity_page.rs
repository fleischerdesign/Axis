use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::cell::RefCell;
use axis_domain::models::continuity::{ContinuityStatus, PeerArrangement, PeerConfig};
use crate::presentation::continuity::{ContinuitySettingsView, ContinuitySettingsPresenter};
use crate::widgets::arrangement_grid::ArrangementGrid;
use crate::widgets::peer_detail_page::PeerDetailPage;
use axis_presentation::View;

pub struct ContinuitySettingsPage {
    root: adw::ToolbarView,
    nav_view: adw::NavigationView,
    enable_switch: adw::SwitchRow,
    peer_list: gtk4::ListBox,
    grid: Rc<ArrangementGrid>,
    current_peer_page: RefCell<Option<Rc<PeerDetailPage>>>,

    toggle_cb: Rc<RefCell<Option<Box<dyn Fn(bool) + 'static>>>>,
    connect_cb: Rc<RefCell<Option<Box<dyn Fn(String) + 'static>>>>,
    disconnect_cb: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    confirm_pin_cb: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    reject_pin_cb: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    cancel_reconnect_cb: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    unpair_cb: Rc<RefCell<Option<Box<dyn Fn(String) + 'static>>>>,
    arrangement_cb: Rc<RefCell<Option<Box<dyn Fn(PeerArrangement) + 'static>>>>,
    config_cb: Rc<RefCell<Option<Box<dyn Fn(String, PeerConfig) + 'static>>>>,
}

impl ContinuitySettingsPage {
    pub fn new(_presenter: Rc<ContinuitySettingsPresenter>) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let main_page = adw::PreferencesPage::builder()
            .title("Continuity")
            .icon_name("input-mouse-symbolic")
            .build();

        let nav_view = adw::NavigationView::new();
        nav_view.add(&adw::NavigationPage::new(&main_page, "Continuity"));
        toolbar_view.set_content(Some(&nav_view));

        let main_group = adw::PreferencesGroup::builder()
            .title("Continuity")
            .description("Multi-device input sharing via network")
            .build();
        main_page.add(&main_group);

        let enable_switch = adw::SwitchRow::builder()
            .title("Enable Continuity")
            .build();
        main_group.add(&enable_switch);

        let arrangement_group = adw::PreferencesGroup::builder()
            .title("Display Arrangement")
            .description("Drag the peer device to position it relative to your screen")
            .build();
        main_page.add(&arrangement_group);

        let grid_cb: Rc<RefCell<Option<Box<dyn Fn(PeerArrangement) + 'static>>>> = Rc::new(RefCell::new(None));
        let grid_cb_closure = grid_cb.clone();
        let grid = ArrangementGrid::new(move |arr| {
            if let Some(f) = grid_cb_closure.borrow().as_ref() {
                f(arr);
            }
        });
        arrangement_group.add(grid.widget());

        let peers_group = adw::PreferencesGroup::builder()
            .title("Devices")
            .build();
        main_page.add(&peers_group);

        let peer_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        peers_group.add(&peer_list);

        let page = Rc::new(Self {
            root: toolbar_view,
            nav_view,
            enable_switch,
            peer_list,
            grid: grid.clone(),
            current_peer_page: RefCell::new(None),
            toggle_cb: Rc::new(RefCell::new(None)),
            connect_cb: Rc::new(RefCell::new(None)),
            disconnect_cb: Rc::new(RefCell::new(None)),
            confirm_pin_cb: Rc::new(RefCell::new(None)),
            reject_pin_cb: Rc::new(RefCell::new(None)),
            cancel_reconnect_cb: Rc::new(RefCell::new(None)),
            unpair_cb: Rc::new(RefCell::new(None)),
            arrangement_cb: grid_cb,
            config_cb: Rc::new(RefCell::new(None)),
        });

        let cb = page.toggle_cb.clone();
        page.enable_switch.connect_active_notify(move |row| {
            if let Some(f) = cb.borrow().as_ref() {
                f(row.is_active());
            }
        });

        page
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }

    fn rebuild_peer_list(&self, status: &ContinuityStatus) {
        while let Some(child) = self.peer_list.first_child() {
            self.peer_list.remove(&child);
        }

        if status.reconnect.is_some() {
            let row = adw::ActionRow::builder()
                .title("Reconnecting...")
                .sensitive(false)
                .build();
            self.peer_list.append(&row);
            return;
        }

        if let Some(pin) = &status.pending_pin {
            if pin.is_incoming {
                let row = adw::ActionRow::builder()
                    .title(&format!("{} wants to connect", pin.peer_name))
                    .subtitle("Pairing request")
                    .build();

                let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                btn_box.set_valign(gtk4::Align::Center);

                let accept_btn = gtk4::Button::builder()
                    .label("Accept")
                    .css_classes(vec!["suggested-action".to_string(), "flat".to_string()])
                    .valign(gtk4::Align::Center)
                    .build();
                let decline_btn = gtk4::Button::builder()
                    .label("Decline")
                    .css_classes(vec!["destructive-action".to_string(), "flat".to_string()])
                    .valign(gtk4::Align::Center)
                    .build();

                let cb_c = self.confirm_pin_cb.clone();
                accept_btn.connect_clicked(move |_| {
                    if let Some(f) = cb_c.borrow().as_ref() { f(); }
                });

                let cb_r = self.reject_pin_cb.clone();
                decline_btn.connect_clicked(move |_| {
                    if let Some(f) = cb_r.borrow().as_ref() { f(); }
                });

                btn_box.append(&accept_btn);
                btn_box.append(&decline_btn);
                row.add_suffix(&btn_box);
                self.peer_list.append(&row);
            }
        }

        if status.peers.is_empty() && status.pending_pin.as_ref().map_or(true, |p| !p.is_incoming) {
            let row = adw::ActionRow::builder()
                .title("No devices found")
                .subtitle("Enable Continuity on other devices to discover them")
                .sensitive(false)
                .build();
            self.peer_list.append(&row);
            return;
        }

        for peer in &status.peers {
            let is_connected = status.active_connection
                .as_ref()
                .map_or(false, |c| c.peer_id == peer.device_id);

            let row = adw::ActionRow::builder()
                .title(&peer.device_name)
                .build();

            if is_connected {
                let connected_secs = status.active_connection
                    .as_ref()
                    .map_or(0, |c| c.connected_secs);
                let time_str = if connected_secs < 60 {
                    format!("{}s ago", connected_secs)
                } else {
                    format!("{}m ago", connected_secs / 60)
                };
                row.set_subtitle(&format!("Connected · {}", time_str));

                let disconnect_btn = gtk4::Button::builder()
                    .label("Disconnect")
                    .css_classes(vec!["destructive-action".to_string(), "flat".to_string()])
                    .valign(gtk4::Align::Center)
                    .build();

                let cb_d = self.disconnect_cb.clone();
                disconnect_btn.connect_clicked(move |_| {
                    if let Some(f) = cb_d.borrow().as_ref() { f(); }
                });
                row.add_suffix(&disconnect_btn);
            } else {
                row.set_subtitle(&peer.hostname);

                let connect_btn = gtk4::Button::builder()
                    .label("Connect")
                    .css_classes(vec!["suggested-action".to_string(), "flat".to_string()])
                    .valign(gtk4::Align::Center)
                    .build();

                let cb_c = self.connect_cb.clone();
                let id_c = peer.device_id.clone();
                connect_btn.connect_clicked(move |_| {
                    if let Some(f) = cb_c.borrow().as_ref() { f(id_c.clone()); }
                });
                row.add_suffix(&connect_btn);
            }

            let peer_id = peer.device_id.clone();
            let peer_name = peer.device_name.clone();
            let nav_view = self.nav_view.clone();
            let config_cb = self.config_cb.clone();
            let disconnect_cb_r = self.disconnect_cb.clone();
            let unpair_cb_r = self.unpair_cb.clone();
            let current_peer_page = self.current_peer_page.clone();
            let gesture = gtk4::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                let detail_page = PeerDetailPage::new(peer_id.clone(), peer_name.clone());

                detail_page.set_on_disconnect({
                    let cb = disconnect_cb_r.clone();
                    Box::new(move || {
                        if let Some(f) = cb.borrow().as_ref() { f(); }
                    })
                });

                detail_page.set_on_unpair({
                    let cb = unpair_cb_r.clone();
                    Box::new(move |id| {
                        if let Some(f) = cb.borrow().as_ref() { f(id); }
                    })
                });

                detail_page.set_on_config({
                    let cb = config_cb.clone();
                    Box::new(move |id, config| {
                        if let Some(f) = cb.borrow().as_ref() { f(id, config); }
                    })
                });

                let nav_page = adw::NavigationPage::new(detail_page.widget(), &peer_name);
                nav_view.push(&nav_page);
                *current_peer_page.borrow_mut() = Some(detail_page);
            });
            row.add_controller(gesture);

            self.peer_list.append(&row);
        }
    }
}

impl View<ContinuityStatus> for ContinuitySettingsPage {
    fn render(&self, status: &ContinuityStatus) {
        self.enable_switch.set_active(status.enabled);
        self.grid.update_status(status);
        if let Some(ref pp) = *self.current_peer_page.borrow() {
            pp.update_status(status);
        }
        self.rebuild_peer_list(status);
    }
}

impl ContinuitySettingsView for ContinuitySettingsPage {
    fn on_toggle_enabled(&self, f: Box<dyn Fn(bool) + 'static>) { *self.toggle_cb.borrow_mut() = Some(f); }
    fn on_connect_peer(&self, f: Box<dyn Fn(String) + 'static>) { *self.connect_cb.borrow_mut() = Some(f); }
    fn on_disconnect(&self, f: Box<dyn Fn() + 'static>) { *self.disconnect_cb.borrow_mut() = Some(f); }
    fn on_confirm_pin(&self, f: Box<dyn Fn() + 'static>) { *self.confirm_pin_cb.borrow_mut() = Some(f); }
    fn on_reject_pin(&self, f: Box<dyn Fn() + 'static>) { *self.reject_pin_cb.borrow_mut() = Some(f); }
    fn on_cancel_reconnect(&self, f: Box<dyn Fn() + 'static>) { *self.cancel_reconnect_cb.borrow_mut() = Some(f); }
    fn on_unpair(&self, f: Box<dyn Fn(String) + 'static>) { *self.unpair_cb.borrow_mut() = Some(f); }
    fn on_set_arrangement(&self, f: Box<dyn Fn(PeerArrangement) + 'static>) { *self.arrangement_cb.borrow_mut() = Some(f); }
    fn on_update_peer_config(&self, f: Box<dyn Fn(String, PeerConfig) + 'static>) { *self.config_cb.borrow_mut() = Some(f); }
}

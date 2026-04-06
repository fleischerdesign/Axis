use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::continuity_proxy::ContinuityProxy;
use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;
use crate::widgets::arrangement_grid::ArrangementGrid;
use axis_core::services::continuity::dbus::ContinuityStateSnapshot;

pub struct ContinuityPage {
    continuity: Option<Rc<ContinuityProxy>>,
    on_peer_clicked: Option<Rc<dyn Fn(String)>>,
}

impl ContinuityPage {
    pub fn new(continuity: Option<&Rc<ContinuityProxy>>) -> Self {
        Self {
            continuity: continuity.cloned(),
            on_peer_clicked: None,
        }
    }

    pub fn with_peer_callback(mut self, cb: impl Fn(String) + 'static) -> Self {
        self.on_peer_clicked = Some(Rc::new(cb));
        self
    }
}

impl SettingsPage for ContinuityPage {
    fn id(&self) -> &'static str { "continuity" }
    fn title(&self) -> &'static str { "Continuity" }
    fn icon(&self) -> &'static str { "input-mouse-symbolic" }

    fn build(&self, proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        let config = proxy.config();
        let updating = Rc::new(Cell::new(false));

        // ── Enable Toggle ───────────────────────────────────────────────
        let main_group = libadwaita::PreferencesGroup::builder()
            .title("Continuity")
            .description("Multi-device input sharing via network")
            .build();

        let enable_row = libadwaita::SwitchRow::builder()
            .title("Enable Continuity")
            .build();
        enable_row.set_active(config.continuity.enabled);

        if let Some(ref cp) = self.continuity {
            let cp_c = cp.clone();
            let updating_c = updating.clone();
            enable_row.connect_active_notify(move |row| {
                if updating_c.get() { return; }
                let p = cp_c.clone();
                let enabled = row.is_active();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_enabled(enabled).await;
                    p.reload();
                });
            });
        } else {
            let proxy_c = proxy.clone();
            let updating_c = updating.clone();
            enable_row.connect_active_notify(move |row| {
                if updating_c.get() { return; }
                let mut cfg = proxy_c.config().continuity;
                cfg.enabled = row.is_active();
                let p = proxy_c.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_continuity(&cfg).await;
                    p.update_cache_continuity(cfg);
                });
            });
        }
        main_group.add(&enable_row);

        // ── Arrangement Grid ────────────────────────────────────────────
        let arrangement_group = libadwaita::PreferencesGroup::builder()
            .title("Display Arrangement")
            .description("Drag devices to position them relative to your screen")
            .build();

        let grid = ArrangementGrid::new(proxy, self.continuity.as_ref());
        arrangement_group.add(grid.widget());

        // ── Devices (unified list with connection status) ───────────────
        let devices_group = libadwaita::PreferencesGroup::builder()
            .title("Devices")
            .build();

        let devices_list = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        devices_group.add(&devices_list);

        // ── Live Updates via Continuity Proxy ───────────────────────────
        if let Some(ref cp) = self.continuity {
            let cp_c = cp.clone();
            // Main update callback
            let devices_list_c = devices_list.clone();
            let on_peer_clicked = self.on_peer_clicked.clone().unwrap_or_else(|| Rc::new(|_| {}));

            let enable_row_c = enable_row.clone();
            let updating_c = updating.clone();
            let cp_inner = cp.clone();
            cp.on_change(move || {
                let state = cp_inner.state();
                updating_c.set(true);
                enable_row_c.set_active(state.enabled);
                updating_c.set(false);
                rebuild_devices_list(&devices_list_c, &state, &cp_inner, on_peer_clicked.clone());
            });

            cp.reload();
        } else {
            let row = libadwaita::ActionRow::builder()
                .title("No devices found")
                .subtitle("Enable Continuity and pair devices via Quick Settings")
                .build();
            row.set_sensitive(false);
            devices_list.append(&row);
        }

        // ── Page Assembly ───────────────────────────────────────────────
        let page = libadwaita::PreferencesPage::new();
        page.add(&main_group);
        page.add(&arrangement_group);
        page.add(&devices_group);
        page.into()
    }
}

// ── Device List Builder ─────────────────────────────────────────────────

fn rebuild_devices_list(
    container: &gtk4::Box,
    state: &ContinuityStateSnapshot,
    cp: &Rc<ContinuityProxy>,
    on_peer_clicked: Rc<dyn Fn(String)>,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    // ── PIN Request Row (if incoming) ───────────────────────────────────
    if let Some(ref pin) = state.pending_pin {
        if pin.is_incoming {
            let pin_row = libadwaita::ActionRow::builder()
                .title(&format!("{} möchte verbinden", pin.peer_name))
                .subtitle("Pairing-Anfrage")
                .build();

            let pin_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            pin_box.set_valign(gtk4::Align::Center);

            let accept_btn = gtk4::Button::builder()
                .label("Accept")
                .css_classes(["suggested-action", "flat"])
                .valign(gtk4::Align::Center)
                .build();
            let decline_btn = gtk4::Button::builder()
                .label("Decline")
                .css_classes(["destructive-action", "flat"])
                .valign(gtk4::Align::Center)
                .build();
            pin_box.append(&accept_btn);
            pin_box.append(&decline_btn);
            pin_row.add_suffix(&pin_box);

            let cp_a = cp.clone();
            accept_btn.connect_clicked(move |_| {
                let p = cp_a.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.confirm_pin().await;
                });
            });

            let cp_d = cp.clone();
            decline_btn.connect_clicked(move |_| {
                let p = cp_d.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.reject_pin().await;
                });
            });

            container.append(&pin_row);
        }
    }

    // ── Peer Rows ───────────────────────────────────────────────────────
    if state.peers.is_empty() && state.pending_pin.as_ref().map_or(true, |p| !p.is_incoming) {
        let row = libadwaita::ActionRow::builder()
            .title("No devices found")
            .subtitle("Make sure Continuity is enabled on other devices")
            .build();
        row.set_sensitive(false);
        container.append(&row);
        return;
    }

    for peer in &state.peers {
        let is_connected = state.active_connection
            .as_ref()
            .map_or(false, |c| c.peer_id == peer.device_id);

        let row = libadwaita::ActionRow::builder()
            .title(&peer.device_name)
            .build();

        // Connection status subtitle
        if is_connected {
            let connected_secs = state.active_connection
                .as_ref()
                .map_or(0, |c| c.connected_secs);
            let time_str = if connected_secs < 60 {
                format!("{}s ago", connected_secs)
            } else {
                format!("{}m ago", connected_secs / 60)
            };
            row.set_subtitle(&format!("Verbunden · {}", time_str));
        } else {
            row.set_subtitle(&peer.hostname);
        }

        // Row click → peer detail (GestureClick since it's in a Box, not ListBox)
        let gesture = gtk4::GestureClick::new();
        let on_click = on_peer_clicked.clone();
        let peer_id = peer.device_id.clone();
        gesture.connect_released(move |_, _, _, _| {
            on_click(peer_id.clone());
        });
        row.add_controller(gesture);

        // Suffix: arrow for all peers
        let arrow = gtk4::Image::from_icon_name("go-next-symbolic");
        arrow.set_valign(gtk4::Align::Center);
        row.add_suffix(&arrow);

        // Disconnect button for connected peers
        if is_connected {
            let disconnect_btn = gtk4::Button::builder()
                .label("Disconnect")
                .css_classes(["destructive-action", "flat"])
                .valign(gtk4::Align::Center)
                .build();

            let cp_d = cp.clone();
            disconnect_btn.connect_clicked(move |_| {
                let p = cp_d.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.disconnect().await;
                });
            });

            // Prevent row click when clicking disconnect
            let stop_gesture = gtk4::GestureClick::new();
            stop_gesture.connect_released(|_, _, _, _| {});
            disconnect_btn.add_controller(stop_gesture);

            row.add_suffix(&disconnect_btn);
        } else {
            let connect_btn = gtk4::Button::builder()
                .label("Connect")
                .css_classes(["suggested-action", "flat"])
                .valign(gtk4::Align::Center)
                .build();

            let cp_c = cp.clone();
            let peer_id_c = peer.device_id.clone();
            connect_btn.connect_clicked(move |_| {
                let p = cp_c.clone();
                let id = peer_id_c.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.connect_to_peer(&id).await;
                });
            });

            // Prevent row click when clicking connect
            let stop_gesture = gtk4::GestureClick::new();
            stop_gesture.connect_released(|_, _, _, _| {});
            connect_btn.add_controller(stop_gesture);

            row.add_suffix(&connect_btn);
        }

        container.append(&row);
    }
}

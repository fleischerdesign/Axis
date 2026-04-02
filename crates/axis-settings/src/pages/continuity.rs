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
}

impl ContinuityPage {
    pub fn new(continuity: Option<&Rc<ContinuityProxy>>) -> Self {
        Self {
            continuity: continuity.cloned(),
        }
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
                    // Reload after toggle to get fresh peer list
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

        // Reactive: update enable switch on external config changes (fallback path)
        if self.continuity.is_none() {
            let enable_row_c = enable_row.clone();
            let updating_c = updating.clone();
            let proxy_c = proxy.clone();
            proxy.on_change(move || {
                let cfg = proxy_c.config();
                updating_c.set(true);
                enable_row_c.set_active(cfg.continuity.enabled);
                updating_c.set(false);
            });
        }

        // ── Status Row (single row: status + action) ────────────────────
        let status_group = libadwaita::PreferencesGroup::builder()
            .title("Connection")
            .build();

        let status_row = libadwaita::ActionRow::builder()
            .title("Disconnected")
            .build();

        let status_action_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        status_action_box.set_valign(gtk4::Align::Center);

        let disconnect_btn = gtk4::Button::builder()
            .label("Disconnect")
            .css_classes(["destructive-action", "flat"])
            .valign(gtk4::Align::Center)
            .visible(false)
            .build();
        status_action_box.append(&disconnect_btn);
        status_row.add_suffix(&status_action_box);
        status_group.add(&status_row);

        // PIN confirmation (hidden by default)
        let pin_row = libadwaita::ActionRow::builder()
            .title("Pairing Request")
            .build();

        let pin_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        pin_box.set_valign(gtk4::Align::Center);

        let pin_confirm_btn = gtk4::Button::builder()
            .label("Accept")
            .css_classes(["suggested-action", "flat"])
            .valign(gtk4::Align::Center)
            .build();
        let pin_reject_btn = gtk4::Button::builder()
            .label("Decline")
            .css_classes(["destructive-action", "flat"])
            .valign(gtk4::Align::Center)
            .build();
        pin_box.append(&pin_confirm_btn);
        pin_box.append(&pin_reject_btn);
        pin_row.add_suffix(&pin_box);
        pin_row.set_visible(false);
        status_group.add(&pin_row);

        // ── Arrangement Grid ────────────────────────────────────────────
        let arrangement_group = libadwaita::PreferencesGroup::builder()
            .title("Display Arrangement")
            .description("Drag devices to position them relative to your screen")
            .build();

        let grid = ArrangementGrid::new(proxy, self.continuity.as_ref());
        arrangement_group.add(grid.widget());

        // ── Devices (single unified list) ───────────────────────────────
        let devices_group = libadwaita::PreferencesGroup::builder()
            .title("Devices")
            .build();

        let devices_list = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        devices_group.add(&devices_list);

        // ── Live Updates via Continuity Proxy ───────────────────────────
        if let Some(ref cp) = self.continuity {
            let cp_c = cp.clone();
            let status_row_c = status_row.clone();
            let disconnect_btn_c = disconnect_btn.clone();
            let pin_row_c = pin_row.clone();
            let devices_list_c = devices_list.clone();

            // Disconnect button
            let cp_d = cp_c.clone();
            disconnect_btn.connect_clicked(move |_| {
                let p = cp_d.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.disconnect().await;
                });
            });

            // PIN buttons
            let cp_pc = cp_c.clone();
            pin_confirm_btn.connect_clicked(move |_| {
                let p = cp_pc.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.confirm_pin().await;
                });
            });
            let cp_pr = cp_c.clone();
            pin_reject_btn.connect_clicked(move |_| {
                let p = cp_pr.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.reject_pin().await;
                });
            });

            // Main update callback
            let proxy_c = proxy.clone();
            let enable_row_c = enable_row.clone();
            let updating_c = updating.clone();
            cp.on_change(move || {
                let state = cp_c.state();
                updating_c.set(true);
                enable_row_c.set_active(state.enabled);
                updating_c.set(false);
                update_status(&status_row_c, &disconnect_btn_c, &state);
                update_pin(&pin_row_c, &state);
                rebuild_devices_list(&devices_list_c, &state, &cp_c, &proxy_c);
            });

            // Ensure fresh data when page is first shown
            cp.reload();
        } else {
            // No continuity proxy — show static persisted peers
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
        page.add(&status_group);
        page.add(&arrangement_group);
        page.add(&devices_group);
        page.into()
    }
}

// ── Update Helpers ──────────────────────────────────────────────────────

fn update_status(
    row: &libadwaita::ActionRow,
    disconnect_btn: &gtk4::Button,
    state: &ContinuityStateSnapshot,
) {
    if let Some(ref conn) = state.active_connection {
        row.set_title(&format!("Connected to {}", conn.peer_name));
        if conn.connected_secs < 60 {
            row.set_subtitle(&format!("{}s ago", conn.connected_secs));
        } else {
            let mins = conn.connected_secs / 60;
            row.set_subtitle(&format!("{}m ago", mins));
        }
        disconnect_btn.set_visible(true);
        disconnect_btn.set_sensitive(true);
    } else {
        row.set_title("Disconnected");
        row.set_subtitle("");
        disconnect_btn.set_visible(false);
    }
}

fn update_pin(row: &libadwaita::ActionRow, state: &ContinuityStateSnapshot) {
    // Only show PIN confirmation for incoming requests
    // (the device that receives the connection request needs to accept/decline)
    if let Some(ref pin) = state.pending_pin {
        if pin.is_incoming {
            row.set_title(&format!("{} wants to connect", pin.peer_name));
            row.set_subtitle("Waiting for your approval");
            row.set_visible(true);
        } else {
            // Outgoing request — we're waiting for the other side to accept
            row.set_visible(false);
        }
    } else {
        row.set_visible(false);
    }
}

fn rebuild_devices_list(
    container: &gtk4::Box,
    state: &ContinuityStateSnapshot,
    cp: &Rc<ContinuityProxy>,
    _settings: &Rc<SettingsProxy>,
) {
    // Clear all children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    if state.peers.is_empty() {
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
            .subtitle(&peer.hostname)
            .build();

        if is_connected {
            let status_icon = gtk4::Image::from_icon_name("emblem-ok-symbolic");
            status_icon.set_tooltip_text(Some("Connected"));
            status_icon.add_css_class("success");
            status_icon.set_valign(gtk4::Align::Center);
            row.add_suffix(&status_icon);
        } else {
            let connect_btn = gtk4::Button::builder()
                .label("Connect")
                .css_classes(["suggested-action", "flat"])
                .valign(gtk4::Align::Center)
                .build();

            let peer_id = peer.device_id.clone();
            let proxy_c = cp.clone();
            connect_btn.connect_clicked(move |_| {
                let p = proxy_c.clone();
                let id = peer_id.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.connect_to_peer(&id).await;
                });
            });
            row.add_suffix(&connect_btn);
        }

        container.append(&row);
    }
}

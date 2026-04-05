use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::continuity_proxy::ContinuityProxy;

/// Detail page for a single paired peer.
/// Not registered in the sidebar — only reached via navigation from the
/// Continuity page's device list.
pub struct PeerDetailPage {
    peer_id: String,
    peer_name: String,
    continuity: Rc<ContinuityProxy>,
}

impl PeerDetailPage {
    pub fn new(peer_id: String, peer_name: String, continuity: Rc<ContinuityProxy>) -> Self {
        Self { peer_id, peer_name, continuity }
    }

    pub fn name(&self) -> &str { &self.peer_name }

    pub fn build(&self) -> gtk4::Widget {
        let updating = Rc::new(Cell::new(false));
        let state = self.continuity.state();

        // ── Capabilities ────────────────────────────────────────────────
        let caps_group = libadwaita::PreferencesGroup::builder()
            .title("Funktionen")
            .build();

        let clipboard_row = libadwaita::SwitchRow::builder()
            .title("Zwischenablage synchronisieren")
            .subtitle("Geteilte Zwischenablage zwischen beiden Geräten")
            .build();

        let audio_row = libadwaita::SwitchRow::builder()
            .title("Audio-Sharing")
            .subtitle("Audio-Wiedergabe an dieses Gerät streamen")
            .build();

        let drag_drop_row = libadwaita::SwitchRow::builder()
            .title("Drag &amp; Drop")
            .subtitle("Dateien per Drag &amp; Drop übertragen")
            .build();

        if let Some(config) = state.peer_configs.get(&self.peer_id) {
            clipboard_row.set_active(config.clipboard);
            audio_row.set_active(config.audio);
            drag_drop_row.set_active(config.drag_drop);
        }

        let cp_c = self.continuity.clone();
        let pid_c = self.peer_id.clone();
        let updating_c = updating.clone();
        clipboard_row.connect_active_notify(move |row| {
            if updating_c.get() { return; }
            let p = cp_c.clone();
            let id = pid_c.clone();
            let val = row.is_active();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.update_peer_config_fields(&id, Some(val), None, None).await;
            });
        });

        let cp_c = self.continuity.clone();
        let pid_c = self.peer_id.clone();
        let updating_c = updating.clone();
        audio_row.connect_active_notify(move |row| {
            if updating_c.get() { return; }
            let p = cp_c.clone();
            let id = pid_c.clone();
            let val = row.is_active();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.update_peer_config_fields(&id, None, Some(val), None).await;
            });
        });

        let cp_c = self.continuity.clone();
        let pid_c = self.peer_id.clone();
        let updating_c = updating.clone();
        drag_drop_row.connect_active_notify(move |row| {
            if updating_c.get() { return; }
            let p = cp_c.clone();
            let id = pid_c.clone();
            let val = row.is_active();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.update_peer_config_fields(&id, None, None, Some(val)).await;
            });
        });

        caps_group.add(&clipboard_row);
        caps_group.add(&audio_row);
        caps_group.add(&drag_drop_row);

        // ── Danger Zone ─────────────────────────────────────────────────
        let danger_group = libadwaita::PreferencesGroup::builder()
            .title("")
            .build();

        let disconnect_btn = gtk4::Button::builder()
            .label("Trennen")
            .css_classes(["destructive-action"])
            .build();

        let unpair_btn = gtk4::Button::builder()
            .label("Entkoppeln")
            .css_classes(["destructive-action"])
            .build();

        let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        btn_box.append(&disconnect_btn);
        btn_box.append(&unpair_btn);
        danger_group.add(&btn_box);

        let is_connected = state.active_connection
            .as_ref()
            .is_some_and(|c| c.peer_id == self.peer_id);
        disconnect_btn.set_visible(is_connected);

        // Disconnect action
        let cp_d = self.continuity.clone();
        disconnect_btn.connect_clicked(move |_| {
            let p = cp_d.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.disconnect().await;
            });
        });

        // Unpair action
        let cp_u = self.continuity.clone();
        let peer_id_clone = self.peer_id.clone();
        unpair_btn.connect_clicked(move |btn| {
            let p = cp_u.clone();
            let id = peer_id_clone.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.unpair(&id).await;
            });
            if let Some(nav_view) = find_navigation_view(btn.upcast_ref::<gtk4::Widget>()) {
                nav_view.pop();
            }
        });

        // ── Page Assembly ───────────────────────────────────────────────
        let page = libadwaita::PreferencesPage::new();
        page.add(&caps_group);
        page.add(&danger_group);

        // Reactive updates
        let cp_c = self.continuity.clone();
        let clipboard_row_c = clipboard_row.clone();
        let audio_row_c = audio_row.clone();
        let drag_drop_row_c = drag_drop_row.clone();
        let disconnect_btn_c = disconnect_btn.clone();
        let updating_c = updating.clone();
        let pid_c = self.peer_id.clone();
        self.continuity.on_change(move || {
            let state = cp_c.state();
            updating_c.set(true);

            if let Some(config) = state.peer_configs.get(&pid_c) {
                clipboard_row_c.set_active(config.clipboard);
                audio_row_c.set_active(config.audio);
                drag_drop_row_c.set_active(config.drag_drop);
            }

            let connected = state.active_connection
                .as_ref()
                .is_some_and(|c| c.peer_id == pid_c);
            disconnect_btn_c.set_visible(connected);

            updating_c.set(false);
        });

        page.into()
    }
}

fn find_navigation_view(widget: &impl gtk4::prelude::IsA<gtk4::Widget>) -> Option<libadwaita::NavigationView> {
    let mut current = widget.upcast_ref().parent();
    while let Some(parent) = current {
        if let Some(nav) = parent.downcast_ref::<libadwaita::NavigationView>() {
            return Some(nav.clone());
        }
        current = parent.parent();
    }
    None
}

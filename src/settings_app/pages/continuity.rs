use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;

pub struct ContinuityPage;

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
        main_group.add(&enable_row);

        // ── Peers List ──────────────────────────────────────────────────
        let peers_group = libadwaita::PreferencesGroup::builder()
            .title("Known Peers")
            .description("Trusted devices for input sharing")
            .build();

        if config.continuity.peer_configs.is_empty() {
            let empty_row = libadwaita::ActionRow::builder()
                .title("No peers configured")
                .subtitle("Pair devices via Quick Settings")
                .build();
            empty_row.set_sensitive(false);
            peers_group.add(&empty_row);
        } else {
            for peer in &config.continuity.peer_configs {
                let row = libadwaita::ExpanderRow::builder()
                    .title(&peer.device_name)
                    .subtitle(&peer.device_id)
                    .build();

                // Clipboard toggle
                let clipboard_row = libadwaita::SwitchRow::builder()
                    .title("Clipboard Sync")
                    .build();
                clipboard_row.set_active(peer.clipboard);

                let proxy_c = proxy.clone();
                let updating_c = updating.clone();
                let peer_id = peer.device_id.clone();
                clipboard_row.connect_active_notify(move |r| {
                    if updating_c.get() { return; }
                    let mut cfg = proxy_c.config().continuity;
                    if let Some(p) = cfg.peer_configs.iter_mut().find(|p| p.device_id == peer_id) {
                        p.clipboard = r.is_active();
                    }
                    let p = proxy_c.clone();
                    gtk4::glib::spawn_future_local(async move {
                        let _ = p.set_continuity(&cfg).await;
                        p.update_cache_continuity(cfg);
                    });
                });
                row.add_row(&clipboard_row);

                // Audio toggle
                let audio_row = libadwaita::SwitchRow::builder()
                    .title("Audio Routing")
                    .build();
                audio_row.set_active(peer.audio);

                let proxy_c = proxy.clone();
                let updating_c = updating.clone();
                let peer_id = peer.device_id.clone();
                audio_row.connect_active_notify(move |r| {
                    if updating_c.get() { return; }
                    let mut cfg = proxy_c.config().continuity;
                    if let Some(p) = cfg.peer_configs.iter_mut().find(|p| p.device_id == peer_id) {
                        p.audio = r.is_active();
                    }
                    let p = proxy_c.clone();
                    gtk4::glib::spawn_future_local(async move {
                        let _ = p.set_continuity(&cfg).await;
                        p.update_cache_continuity(cfg);
                    });
                });
                row.add_row(&audio_row);

                // Drag & Drop toggle
                let dnd_row = libadwaita::SwitchRow::builder()
                    .title("Drag & Drop")
                    .build();
                dnd_row.set_active(peer.drag_drop);

                let proxy_c = proxy.clone();
                let updating_c = updating.clone();
                let peer_id = peer.device_id.clone();
                dnd_row.connect_active_notify(move |r| {
                    if updating_c.get() { return; }
                    let mut cfg = proxy_c.config().continuity;
                    if let Some(p) = cfg.peer_configs.iter_mut().find(|p| p.device_id == peer_id) {
                        p.drag_drop = r.is_active();
                    }
                    let p = proxy_c.clone();
                    gtk4::glib::spawn_future_local(async move {
                        let _ = p.set_continuity(&cfg).await;
                        p.update_cache_continuity(cfg);
                    });
                });
                row.add_row(&dnd_row);

                peers_group.add(&row);
            }
        }

        let page = libadwaita::PreferencesPage::new();
        page.add(&main_group);
        page.add(&peers_group);
        page.into()
    }
}

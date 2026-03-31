use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;
use crate::config::*;

pub struct BarPage;

impl SettingsPage for BarPage {
    fn id(&self) -> &'static str { "bar" }
    fn title(&self) -> &'static str { "Bar" }
    fn icon(&self) -> &'static str { "preferences-desktop-panels-symbolic" }

    fn build(&self, proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        let _group = libadwaita::PreferencesGroup::builder()
            .title("Bar")
            .description("Configure the panel bar position, behavior, and visible sections")
            .build();

        let config = proxy.config();
        let updating = Rc::new(Cell::new(false));

        // ── Position ────────────────────────────────────────────────────
        let pos_group = libadwaita::PreferencesGroup::builder()
            .title("Position")
            .build();

        let pos_bottom = gtk4::CheckButton::with_label("Bottom");
        let pos_top = gtk4::CheckButton::with_label("Top");
        pos_top.set_group(Some(&pos_bottom));

        match config.bar.position {
            BarPosition::Bottom => pos_bottom.set_active(true),
            BarPosition::Top => pos_top.set_active(true),
        }

        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        pos_bottom.connect_toggled(move |btn| {
            if !btn.is_active() || updating_c.get() { return; }
            let mut cfg = proxy_c.config().bar;
            cfg.position = BarPosition::Bottom;
            let json = serde_json::to_string(&cfg).unwrap();
            let p = proxy_c.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.set_bar(&serde_json::from_str(&json).unwrap()).await;
                p.update_cache_bar(serde_json::from_str(&json).unwrap());
            });
        });

        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        pos_top.connect_toggled(move |btn| {
            if !btn.is_active() || updating_c.get() { return; }
            let mut cfg = proxy_c.config().bar;
            cfg.position = BarPosition::Top;
            let p = proxy_c.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.set_bar(&cfg).await;
                p.update_cache_bar(cfg);
            });
        });

        let pos_row = libadwaita::ActionRow::builder().title("Position").build();
        let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        btn_box.set_valign(gtk4::Align::Center);
        btn_box.append(&pos_bottom);
        btn_box.append(&pos_top);
        pos_row.add_suffix(&btn_box);
        pos_group.add(&pos_row);

        // ── Autohide ────────────────────────────────────────────────────
        let autohide_row = libadwaita::SwitchRow::builder()
            .title("Autohide")
            .subtitle("Hide bar when not hovered or no popup is open")
            .build();
        autohide_row.set_active(config.bar.autohide);
        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        autohide_row.connect_active_notify(move |row| {
            if updating_c.get() { return; }
            let mut cfg = proxy_c.config().bar;
            cfg.autohide = row.is_active();
            let p = proxy_c.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.set_bar(&cfg).await;
                p.update_cache_bar(cfg);
            });
        });
        pos_group.add(&autohide_row);

        // ── Layer ───────────────────────────────────────────────────────
        let layer_row = libadwaita::ComboRow::builder()
            .title("Layer")
            .model(&gtk4::StringList::new(&["Top", "Bottom", "Overlay"]))
            .build();
        let layer_idx = match config.bar.layer {
            BarLayer::Top => 0,
            BarLayer::Bottom => 1,
            BarLayer::Overlay => 2,
        };
        layer_row.set_selected(layer_idx);
        let proxy_c = proxy.clone();
        let updating_c = updating.clone();
        layer_row.connect_selected_notify(move |row| {
            if updating_c.get() { return; }
            let layer = match row.selected() {
                0 => BarLayer::Top,
                1 => BarLayer::Bottom,
                _ => BarLayer::Overlay,
            };
            let mut cfg = proxy_c.config().bar;
            cfg.layer = layer;
            let p = proxy_c.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.set_bar(&cfg).await;
                p.update_cache_bar(cfg);
            });
        });
        pos_group.add(&layer_row);

        // ── Islands ─────────────────────────────────────────────────────
        let islands_group = libadwaita::PreferencesGroup::builder()
            .title("Islands")
            .description("Toggle visibility of bar sections")
            .build();

        for (label, field) in [
            ("Launcher", "launcher"),
            ("Clock", "clock"),
            ("Status", "status"),
            ("Workspaces", "workspace"),
        ] {
            let row = libadwaita::SwitchRow::builder().title(label).build();
            let active = match field {
                "launcher" => config.bar.islands.launcher,
                "clock" => config.bar.islands.clock,
                "status" => config.bar.islands.status,
                "workspace" => config.bar.islands.workspace,
                _ => true,
            };
            row.set_active(active);

            let proxy_c = proxy.clone();
            let updating_c = updating.clone();
            let field = field.to_string();
            row.connect_active_notify(move |r| {
                if updating_c.get() { return; }
                let mut cfg = proxy_c.config().bar;
                match field.as_str() {
                    "launcher" => cfg.islands.launcher = r.is_active(),
                    "clock" => cfg.islands.clock = r.is_active(),
                    "status" => cfg.islands.status = r.is_active(),
                    "workspace" => cfg.islands.workspace = r.is_active(),
                    _ => {}
                }
                let p = proxy_c.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_bar(&cfg).await;
                    p.update_cache_bar(cfg);
                });
            });
            islands_group.add(&row);
        }

        let page = libadwaita::PreferencesPage::new();
        page.add(&pos_group);
        page.add(&islands_group);

        // Reactive: update widgets on external config changes
        let autohide_row_c = autohide_row.clone();
        let updating_c = updating.clone();
        let proxy_c = proxy.clone();
        proxy.on_change(move || {
            let cfg = proxy_c.config();
            updating_c.set(true);
            autohide_row_c.set_active(cfg.bar.autohide);
            updating_c.set(false);
        });

        page.into()
    }
}

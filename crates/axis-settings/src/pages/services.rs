use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;

pub struct ServicesPage;

impl SettingsPage for ServicesPage {
    fn id(&self) -> &'static str { "services" }
    fn title(&self) -> &'static str { "Services" }
    fn icon(&self) -> &'static str { "preferences-system-symbolic" }

    fn build(&self, proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        let config = proxy.config();
        let updating = Rc::new(Cell::new(false));

        let toggle_group = libadwaita::PreferencesGroup::builder()
            .title("Background Services")
            .description("Enable or disable shell services")
            .build();

        let row_specs = [("Bluetooth", "bluetooth"), ("Airplane Mode", "airplane"), ("Do Not Disturb", "dnd")];
        let rows: Rc<Vec<(String, libadwaita::SwitchRow)>> = Rc::new(
            row_specs.iter().map(|(label, field)| {
                let row = libadwaita::SwitchRow::builder().title(*label).build();
                let active = match *field {
                    "bluetooth" => config.services.bluetooth_enabled,
                    "airplane" => config.services.airplane_enabled,
                    "dnd" => config.services.dnd_enabled,
                    _ => false,
                };
                row.set_active(active);

                let proxy_c = proxy.clone();
                let updating_c = updating.clone();
                let f = field.to_string();
                row.connect_active_notify(move |r| {
                    if updating_c.get() { return; }
                    let mut cfg = proxy_c.config().services;
                    match f.as_str() {
                        "bluetooth" => cfg.bluetooth_enabled = r.is_active(),
                        "airplane" => cfg.airplane_enabled = r.is_active(),
                        "dnd" => cfg.dnd_enabled = r.is_active(),
                        _ => {}
                    }
                    let p = proxy_c.clone();
                    gtk4::glib::spawn_future_local(async move {
                        let _ = p.set_services(&cfg).await;
                        p.update_cache_services(cfg);
                    });
                });
                toggle_group.add(&row);
                (field.to_string(), row)
            }).collect()
        );

        // Reactive: update switches on external config changes
        let rows_c = rows.clone();
        let updating_c = updating.clone();
        let proxy_c = proxy.clone();
        proxy.on_change(move || {
            let cfg = proxy_c.config();
            updating_c.set(true);
            for (field, row) in rows_c.iter() {
                match field.as_str() {
                    "bluetooth" => row.set_active(cfg.services.bluetooth_enabled),
                    "airplane" => row.set_active(cfg.services.airplane_enabled),
                    "dnd" => row.set_active(cfg.services.dnd_enabled),
                    _ => {}
                }
            }
            updating_c.set(false);
        });

        let page = libadwaita::PreferencesPage::new();
        page.add(&toggle_group);
        page.into()
    }
}

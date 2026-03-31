use std::cell::Cell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;
use crate::config::*;

pub struct ShortcutsPage;

impl SettingsPage for ShortcutsPage {
    fn id(&self) -> &'static str { "shortcuts" }
    fn title(&self) -> &'static str { "Shortcuts" }
    fn icon(&self) -> &'static str { "preferences-desktop-keyboard-symbolic" }

    fn build(&self, proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        let config = proxy.config();
        let updating = Rc::new(Cell::new(false));

        let group = libadwaita::PreferencesGroup::builder()
            .title("Keyboard Shortcuts")
            .description("Configure global keyboard shortcuts")
            .build();

        // Collect widget references for on_change reactivity
        let mut shortcut_labels: Vec<gtk4::Label> = Vec::new();
        let mut shortcut_entries: Vec<gtk4::Entry> = Vec::new();

        for (label, _default_val, field) in [
            ("Open Launcher", "<Super>space", "launcher"),
            ("Open Quick Settings", "<Super>s", "quick_settings"),
            ("Open Workspaces", "<Super>w", "workspaces"),
            ("Lock Screen", "<Super>l", "lock"),
        ] {
            let current = match field {
                "launcher" => &config.shortcuts.launcher,
                "quick_settings" => &config.shortcuts.quick_settings,
                "workspaces" => &config.shortcuts.workspaces,
                "lock" => &config.shortcuts.lock,
                _ => _default_val,
            };

            let row = libadwaita::ActionRow::builder()
                .title(label)
                .build();

            // Current shortcut display
            let shortcut_label = gtk4::Label::new(Some(current));
            shortcut_label.add_css_class("dim-label");
            shortcut_label.add_css_class("monospace");
            row.add_suffix(&shortcut_label);

            // Entry for editing (hidden by default, shown on click)
            let entry = gtk4::Entry::builder()
                .text(current)
                .width_chars(15)
                .build();
            entry.set_visible(false);
            row.add_suffix(&entry);

            // Edit button
            let edit_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
            edit_btn.add_css_class("flat");
            edit_btn.set_valign(gtk4::Align::Center);

            let shortcut_label_c = shortcut_label.clone();
            let entry_c = entry.clone();
            edit_btn.connect_clicked(move |_| {
                let is_editing = entry_c.property::<bool>("visible");
                shortcut_label_c.set_visible(is_editing);
                entry_c.set_visible(!is_editing);
                if !is_editing {
                    entry_c.grab_focus();
                }
            });
            row.add_suffix(&edit_btn);
            row.set_activatable(false);

            // Apply on Enter
            let proxy_c = proxy.clone();
            let updating_c = updating.clone();
            let shortcut_label_c = shortcut_label.clone();
            let field = field.to_string();
            entry.connect_activate(move |e| {
                if updating_c.get() { return; }
                let val = e.text().to_string();
                shortcut_label_c.set_text(&val);
                shortcut_label_c.set_visible(true);
                e.set_visible(false);

                let mut cfg = proxy_c.config().shortcuts;
                match field.as_str() {
                    "launcher" => cfg.launcher = val,
                    "quick_settings" => cfg.quick_settings = val,
                    "workspaces" => cfg.workspaces = val,
                    "lock" => cfg.lock = val,
                    _ => {}
                }
                let p = proxy_c.clone();
                gtk4::glib::spawn_future_local(async move {
                    let _ = p.set_shortcuts(&cfg).await;
                    p.update_cache_shortcuts(cfg);
                });
            });

            // Cancel on Escape
            let shortcut_label_c = shortcut_label.clone();
            let entry_c = entry.clone();
            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, key, _, _| {
                if key == gtk4::gdk::Key::Escape {
                    shortcut_label_c.set_visible(true);
                    entry_c.set_visible(false);
                    return gtk4::glib::Propagation::Stop;
                }
                gtk4::glib::Propagation::Proceed
            });
            entry.add_controller(key_controller);

            shortcut_labels.push(shortcut_label);
            shortcut_entries.push(entry);

            group.add(&row);
        }

        // Reset All button
        let reset_row = libadwaita::ActionRow::builder().build();
        let reset_btn = gtk4::Button::with_label("Reset All to Defaults");
        reset_btn.add_css_class("destructive-action");
        reset_btn.set_valign(gtk4::Align::Center);

        let proxy_c = proxy.clone();
        reset_btn.connect_clicked(move |_| {
            let default = ShortcutsConfig::default();
            let p = proxy_c.clone();
            gtk4::glib::spawn_future_local(async move {
                let _ = p.set_shortcuts(&default).await;
                p.update_cache_shortcuts(default);
            });
        });
        reset_row.add_suffix(&reset_btn);
        group.add(&reset_row);

        let page = libadwaita::PreferencesPage::new();
        page.add(&group);

        // Reactive: update widgets on external config changes
        let updating_c = updating.clone();
        let proxy_c = proxy.clone();
        proxy.on_change(move || {
            let cfg = proxy_c.config();
            updating_c.set(true);

            let values = [
                cfg.shortcuts.launcher.as_str(),
                cfg.shortcuts.quick_settings.as_str(),
                cfg.shortcuts.workspaces.as_str(),
                cfg.shortcuts.lock.as_str(),
            ];

            for (i, val) in values.iter().enumerate() {
                if let Some(label) = shortcut_labels.get(i) {
                    label.set_text(val);
                    label.set_visible(true);
                }
                if let Some(entry) = shortcut_entries.get(i) {
                    entry.set_text(val);
                    entry.set_visible(false);
                }
            }

            updating_c.set(false);
        });

        page.into()
    }
}

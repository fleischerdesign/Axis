use crate::app_context::AppContext;
use crate::widgets::Island;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

// Gesamthöhe des Bar-Fensters (Inhalt + Abstand nach unten)
const BAR_HEIGHT: i32 = 54;
// Wie viele Pixel sichtbar bleiben wenn versteckt (>0 damit Pointer-Events ankommen)
const PEEK_PX: i32 = 1;
// Verzögerung bevor die Bar wieder verschwindet
const HIDE_DELAY_MS: u64 = 300;
// Animations-Framerate (16ms ≈ 60fps)
const ANIM_INTERVAL_MS: u64 = 16;
// Pixel pro Frame
const ANIM_STEP: i32 = 8;

pub struct Bar {
    pub window: gtk4::ApplicationWindow,
    pub status_island: gtk4::Box,
    pub center_island: gtk4::Box,
    pub vol_icon: gtk4::Image,
    pub popup_open: Rc<RefCell<bool>>,
}

impl Bar {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Self {
        let is_visible: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let hide_timeout: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
        let anim_source: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
        let popup_open: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Carp Bottom Bar")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        // exclusive_zone(-1): Fenster darf außerhalb des nutzbaren Bereichs liegen
        window.set_exclusive_zone(-1);
        // Versteckt starten: fast komplett off-screen, nur PEEK_PX sichtbar
        window.set_margin(Edge::Bottom, -(BAR_HEIGHT - PEEK_PX));

        let root = gtk4::CenterBox::new();
        root.set_margin_bottom(10);
        root.set_height_request(44);

        // --- 1. Launcher ---
        let launcher_island = Island::new(0);
        launcher_island.append(&gtk4::Image::from_icon_name("view-app-grid-symbolic"));
        root.set_start_widget(Some(&launcher_island.container));

        // --- 2. Center (Workspace & Clock) ---
        let center_island = Island::new(12);
        center_island
            .container
            .set_cursor_from_name(Some("pointer"));
        let ws_label = gtk4::Label::new(None);
        ws_label.add_css_class("workspace-label");
        let clock_label = gtk4::Label::new(None);
        clock_label.add_css_class("clock-label");
        center_island.append(&ws_label);
        center_island.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        center_island.append(&clock_label);
        root.set_center_widget(Some(&center_island.container));

        // --- 3. Status ---
        let status_island = Island::new(12);
        status_island
            .container
            .set_cursor_from_name(Some("pointer"));

        let wifi_icon = gtk4::Image::from_icon_name("network-wireless-symbolic");
        let bt_icon = gtk4::Image::from_icon_name("bluetooth-symbolic");
        let vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
        let battery_icon = gtk4::Image::from_icon_name("battery-full-symbolic");

        status_island.append(&wifi_icon);
        status_island.append(&bt_icon);
        status_island.append(&vol_icon);
        status_island.append(&battery_icon);

        root.set_end_widget(Some(&status_island.container));
        window.set_child(Some(&root));

        // --- AUTO-HIDE: slide-in beim Hovern ---
        let motion = gtk4::EventControllerMotion::new();

        {
            let window_ref = window.clone();
            let is_visible_ref = is_visible.clone();
            let hide_timeout_ref = hide_timeout.clone();
            let anim_source_ref = anim_source.clone();

            motion.connect_enter(move |_, _x, _y| {
                // Laufenden Hide-Timer abbrechen
                if let Some(src) = hide_timeout_ref.borrow_mut().take() {
                    src.remove();
                }
                // Bereits sichtbar oder Animation läuft schon → nichts tun
                if *is_visible_ref.borrow() || anim_source_ref.borrow().is_some() {
                    return;
                }
                *is_visible_ref.borrow_mut() = true;

                // Slide-in: Margin von -(BAR_HEIGHT - PEEK_PX) → 0
                let window_anim = window_ref.clone();
                let anim_source_cb = anim_source_ref.clone();
                let src =
                    glib::timeout_add_local(Duration::from_millis(ANIM_INTERVAL_MS), move || {
                        let current = window_anim.margin(Edge::Bottom);
                        let next = (current + ANIM_STEP).min(0);
                        window_anim.set_margin(Edge::Bottom, next);
                        if next >= 0 {
                            *anim_source_cb.borrow_mut() = None;
                            glib::ControlFlow::Break
                        } else {
                            glib::ControlFlow::Continue
                        }
                    });
                *anim_source_ref.borrow_mut() = Some(src);
            });
        }

        {
            let window_ref = window.clone();
            let is_visible_ref = is_visible.clone();
            let hide_timeout_ref = hide_timeout.clone();
            let anim_source_ref = anim_source.clone();
            let popup_open_ref = popup_open.clone();

            motion.connect_leave(move |_| {
                // Popup ist offen → Bar darf nicht einklappen
                if *popup_open_ref.borrow() {
                    return;
                }
                // Schon ein Hide-Timer aktiv → nichts tun
                if hide_timeout_ref.borrow().is_some() {
                    return;
                }
                let window_for_cb = window_ref.clone();
                let is_visible_for_cb = is_visible_ref.clone();
                let hide_timeout_for_cb = hide_timeout_ref.clone();
                let anim_source_for_cb = anim_source_ref.clone();

                let src =
                    glib::timeout_add_local_once(Duration::from_millis(HIDE_DELAY_MS), move || {
                        *is_visible_for_cb.borrow_mut() = false;
                        *hide_timeout_for_cb.borrow_mut() = None;

                        // Laufende Slide-in Animation abbrechen
                        if let Some(anim) = anim_source_for_cb.borrow_mut().take() {
                            anim.remove();
                        }

                        // Slide-out: Margin von aktuellem Wert → -(BAR_HEIGHT - PEEK_PX)
                        let window_anim = window_for_cb.clone();
                        let anim_source_cb = anim_source_for_cb.clone();
                        let src = glib::timeout_add_local(
                            Duration::from_millis(ANIM_INTERVAL_MS),
                            move || {
                                let current = window_anim.margin(Edge::Bottom);
                                let target = -(BAR_HEIGHT - PEEK_PX);
                                let next = (current - ANIM_STEP).max(target);
                                window_anim.set_margin(Edge::Bottom, next);
                                if next <= target {
                                    *anim_source_cb.borrow_mut() = None;
                                    glib::ControlFlow::Break
                                } else {
                                    glib::ControlFlow::Continue
                                }
                            },
                        );
                        *anim_source_for_cb.borrow_mut() = Some(src);
                    });
                *hide_timeout_ref.borrow_mut() = Some(src);
            });
        }

        window.add_controller(motion);

        // --- REAKTIVE BINDINGS ---

        ctx.clock.subscribe(move |time| {
            clock_label.set_text(&time.format("%H:%M").to_string());
        });

        ctx.niri.subscribe(move |data| {
            Self::update_workspaces(&ws_label, data);
        });

        ctx.network.subscribe(move |data| {
            Self::update_wifi(&wifi_icon, data);
        });

        ctx.bluetooth.subscribe(move |data| {
            Self::update_bluetooth(&bt_icon, data);
        });

        ctx.power.subscribe(move |data| {
            Self::update_battery(&battery_icon, data);
        });

        let vol_icon_clone = vol_icon.clone();
        ctx.audio.subscribe(move |data| {
            Self::update_volume(&vol_icon_clone, data);
        });

        Self {
            window,
            status_island: status_island.container,
            center_island: center_island.container,
            vol_icon,
            popup_open,
        }
    }

    fn update_workspaces(label: &gtk4::Label, data: &crate::services::niri::NiriData) {
        let mut workspaces = data.workspaces.clone();
        workspaces.sort_by_key(|w| w.id);
        let mut markup = String::new();
        for ws in workspaces {
            if ws.is_active {
                markup.push_str(&format!(" <b>{}</b> ", ws.id));
            } else {
                markup.push_str(&format!(" {} ", ws.id));
            }
        }
        label.set_markup(&markup);
    }

    fn update_wifi(icon: &gtk4::Image, data: &crate::services::network::NetworkData) {
        icon.set_visible(data.is_wifi_enabled);
        if data.is_wifi_enabled {
            let icon_name = if !data.is_wifi_connected {
                "network-wireless-offline-symbolic"
            } else if data.active_strength > 80 {
                "network-wireless-signal-excellent-symbolic"
            } else if data.active_strength > 60 {
                "network-wireless-signal-good-symbolic"
            } else if data.active_strength > 40 {
                "network-wireless-signal-ok-symbolic"
            } else {
                "network-wireless-signal-weak-symbolic"
            };
            icon.set_icon_name(Some(icon_name));
        }
    }

    fn update_bluetooth(icon: &gtk4::Image, data: &crate::services::bluetooth::BluetoothData) {
        icon.set_visible(data.is_powered);
        if data.is_powered {
            let any_connected = data.devices.iter().any(|d| d.is_connected);
            let icon_name = if any_connected {
                "bluetooth-active-symbolic"
            } else {
                "bluetooth-symbolic"
            };
            icon.set_icon_name(Some(icon_name));
        }
    }

    fn update_battery(icon: &gtk4::Image, data: &crate::services::power::PowerData) {
        icon.set_visible(data.has_battery);
        if data.has_battery {
            let icon_name = if data.is_charging {
                "battery-full-charging-symbolic"
            } else if data.battery_percentage < 10.0 {
                "battery-empty-symbolic"
            } else if data.battery_percentage < 30.0 {
                "battery-low-symbolic"
            } else if data.battery_percentage < 60.0 {
                "battery-good-symbolic"
            } else {
                "battery-full-symbolic"
            };
            icon.set_icon_name(Some(icon_name));
        }
    }

    fn update_volume(icon: &gtk4::Image, data: &crate::services::audio::AudioData) {
        let icon_name = if data.is_muted || data.volume <= 0.01 {
            "audio-volume-muted-symbolic"
        } else if data.volume < 0.33 {
            "audio-volume-low-symbolic"
        } else if data.volume < 0.66 {
            "audio-volume-medium-symbolic"
        } else {
            "audio-volume-high-symbolic"
        };
        icon.set_icon_name(Some(icon_name));
    }
}

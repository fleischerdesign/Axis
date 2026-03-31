use crate::services::niri::NiriService;
use gtk4::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use zbus;

pub struct PowerActionStack {
    pub stack: gtk4::Stack,
    pub power_expanded: Rc<Cell<bool>>,
}

impl PowerActionStack {
    pub fn new(on_lock: Rc<dyn Fn()>) -> Self {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .transition_duration(200)
            .build();

        let normal_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let screenshot_btn = Self::create_btn("camera-photo-symbolic");
        let settings_btn = Self::create_btn("emblem-system-symbolic");
        settings_btn.set_sensitive(false);
        settings_btn.set_tooltip_text(Some("Coming soon"));
        let lock_btn = Self::create_btn("system-lock-screen-symbolic");
        lock_btn.set_tooltip_text(Some("Lock Screen"));
        let power_btn = Self::create_btn("system-shutdown-symbolic");

        normal_actions.append(&screenshot_btn);
        normal_actions.append(&settings_btn);
        normal_actions.append(&lock_btn);
        normal_actions.append(&power_btn);

        let power_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let sleep_btn = Self::create_power_btn("media-playback-pause-symbolic", "sleep");
        sleep_btn.set_tooltip_text(Some("Sleep"));
        let shutdown_btn = Self::create_power_btn("system-shutdown-symbolic", "shutdown");
        shutdown_btn.set_tooltip_text(Some("Shut Down"));
        let restart_btn = Self::create_power_btn("system-reboot-symbolic", "restart");
        restart_btn.set_tooltip_text(Some("Restart"));
        let close_btn = Self::create_btn("window-close-symbolic");

        power_actions.append(&sleep_btn);
        power_actions.append(&shutdown_btn);
        power_actions.append(&restart_btn);
        power_actions.append(&close_btn);

        stack.add_named(&normal_actions, Some("normal"));
        stack.add_named(&power_actions, Some("power"));

        let power_expanded = Rc::new(Cell::new(false));

        // Screenshot
        screenshot_btn.connect_clicked(move |_| {
            NiriService::spawn_action(niri_ipc::Action::Screenshot {
                show_pointer: false,
                path: None,
            });
        });

        // Lock Screen
        lock_btn.connect_clicked(move |_| {
            on_lock();
        });

        // Power expand/collapse
        let stack_expand = stack.clone();
        let power_expanded_c = power_expanded.clone();
        power_btn.connect_clicked(move |_| {
            stack_expand.set_visible_child_name("power");
            power_expanded_c.set(true);
        });

        let stack_collapse = stack.clone();
        let power_expanded_c = power_expanded.clone();
        close_btn.connect_clicked(move |_| {
            stack_collapse.set_visible_child_name("normal");
            power_expanded_c.set(false);
        });

        // Power actions via D-Bus logind
        sleep_btn.connect_clicked(move |_| {
            Self::power_action("Suspend");
        });
        shutdown_btn.connect_clicked(move |_| {
            Self::power_action("PowerOff");
        });
        restart_btn.connect_clicked(move |_| {
            Self::power_action("Reboot");
        });

        Self {
            stack,
            power_expanded,
        }
    }

    pub fn is_power_expanded(&self) -> bool {
        self.power_expanded.get()
    }

    pub fn collapse_power_menu(&self) {
        self.stack.set_visible_child_name("normal");
        self.power_expanded.set(false);
    }

    fn create_btn(icon: &str) -> gtk4::Button {
        gtk4::Button::builder()
            .icon_name(icon)
            .css_classes(vec!["qs-bottom-btn".to_string()])
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build()
    }

    fn create_power_btn(icon: &str, css_class: &str) -> gtk4::Button {
        gtk4::Button::builder()
            .icon_name(icon)
            .css_classes(vec![
                "qs-power-btn".to_string(),
                css_class.to_string(),
            ])
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build()
    }

    fn power_action(method: &str) {
        let method = method.to_string();
        gtk4::glib::spawn_future_local(async move {
            if let Ok(conn) = zbus::Connection::system().await {
                let _ = conn
                    .call_method(
                        Some("org.freedesktop.login1"),
                        "/org/freedesktop/login1",
                        Some("org.freedesktop.login1.Manager"),
                        method.as_str(),
                        &(true,),
                    )
                    .await;
            }
        });
    }
}

use gtk4::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use axis_application::use_cases::power::suspend::SuspendUseCase;
use axis_application::use_cases::power::power_off::PowerOffUseCase;
use axis_application::use_cases::power::reboot::RebootUseCase;
use axis_application::use_cases::lock::lock::LockSessionUseCase;

pub struct PowerActionStack {
    pub stack: gtk4::Stack,
    pub power_expanded: Rc<Cell<bool>>,
}

impl std::fmt::Debug for PowerActionStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PowerActionStack")
            .field("power_expanded", &self.power_expanded.get())
            .finish()
    }
}

impl PowerActionStack {
    pub fn new(
        suspend_uc: Arc<SuspendUseCase>,
        power_off_uc: Arc<PowerOffUseCase>,
        reboot_uc: Arc<RebootUseCase>,
        lock_session_uc: Arc<LockSessionUseCase>,
    ) -> Self {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .transition_duration(200)
            .build();

        let normal_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        let screenshot_btn = Self::create_btn("camera-photo-symbolic");
        let settings_btn = Self::create_btn("emblem-system-symbolic");
        settings_btn.set_tooltip_text(Some("Axis Settings"));
        let lock_btn = Self::create_btn("system-lock-screen-symbolic");
        lock_btn.set_tooltip_text(Some("Lock Screen"));
        let power_btn = Self::create_btn("system-shutdown-symbolic");

        normal_actions.append(&screenshot_btn);
        normal_actions.append(&settings_btn);
        normal_actions.append(&lock_btn);
        normal_actions.append(&power_btn);

        let power_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

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

        screenshot_btn.connect_clicked(move |_| {
            tokio::spawn(async move {
                let _ = tokio::process::Command::new("niri")
                    .args(["msg", "action", "screenshot"])
                    .status()
                    .await;
            });
        });

        settings_btn.connect_clicked(move |_| {
            match std::process::Command::new("axis-settings").spawn() {
                Ok(_) => log::info!("[qs] Settings app launched"),
                Err(e) => log::warn!("[qs] Failed to launch settings app: {e}"),
            }
        });

        {
            let uc = lock_session_uc.clone();
            lock_btn.connect_clicked(move |_| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    let _ = uc.execute().await;
                });
            });
        }

        {
            let stack_c = stack.clone();
            let pe_c = power_expanded.clone();
            power_btn.connect_clicked(move |_| {
                stack_c.set_visible_child_name("power");
                pe_c.set(true);
            });
        }

        {
            let stack_c = stack.clone();
            let pe_c = power_expanded.clone();
            close_btn.connect_clicked(move |_| {
                stack_c.set_visible_child_name("normal");
                pe_c.set(false);
            });
        }

        {
            let uc = suspend_uc.clone();
            sleep_btn.connect_clicked(move |_| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    let _ = uc.execute().await;
                });
            });
        }

        {
            let uc = power_off_uc.clone();
            shutdown_btn.connect_clicked(move |_| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    let _ = uc.execute().await;
                });
            });
        }

        {
            let uc = reboot_uc.clone();
            restart_btn.connect_clicked(move |_| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    let _ = uc.execute().await;
                });
            });
        }

        Self { stack, power_expanded }
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
}

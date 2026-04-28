use std::sync::Arc;
use axis_application::use_cases::power::subscribe::SubscribeToPowerUpdatesUseCase;
use axis_domain::models::power::PowerStatus;
use axis_presentation::{Presenter, View};

pub(crate) fn battery_icon(percentage: f64, charging: bool) -> &'static str {
    let level = ((percentage / 10.0).round() * 10.0) as u32;
    let level = level.min(100);

    if charging && level >= 100 {
        "battery-level-100-charged-symbolic"
    } else if charging {
        match level {
            0 => "battery-level-0-charging-symbolic",
            10 => "battery-level-10-charging-symbolic",
            20 => "battery-level-20-charging-symbolic",
            30 => "battery-level-30-charging-symbolic",
            40 => "battery-level-40-charging-symbolic",
            50 => "battery-level-50-charging-symbolic",
            60 => "battery-level-60-charging-symbolic",
            70 => "battery-level-70-charging-symbolic",
            80 => "battery-level-80-charging-symbolic",
            90 => "battery-level-90-charging-symbolic",
            _ => "battery-level-100-charging-symbolic",
        }
    } else {
        match level {
            0 => "battery-level-0-symbolic",
            10 => "battery-level-10-symbolic",
            20 => "battery-level-20-symbolic",
            30 => "battery-level-30-symbolic",
            40 => "battery-level-40-symbolic",
            50 => "battery-level-50-symbolic",
            60 => "battery-level-60-symbolic",
            70 => "battery-level-70-symbolic",
            80 => "battery-level-80-symbolic",
            90 => "battery-level-90-symbolic",
            _ => "battery-level-100-symbolic",
        }
    }
}

pub struct BatteryPresenter {
    inner: Presenter<PowerStatus>,
}

impl BatteryPresenter {
    pub fn new(use_case: Arc<SubscribeToPowerUpdatesUseCase>) -> Self {
        let inner = Presenter::from_subscribe({
            let uc = use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        });
        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<PowerStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub async fn bind(&self, view: Box<dyn View<PowerStatus>>) {
        self.inner.bind(view).await;
    }
}

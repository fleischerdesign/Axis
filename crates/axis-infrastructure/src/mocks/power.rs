use async_trait::async_trait;
use axis_domain::models::power::{PowerProfile, PowerStatus};
use axis_domain::ports::power::{PowerError, PowerProvider, PowerStream};
use log::info;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct MockPowerProvider {
    status_tx: watch::Sender<PowerStatus>,
}

impl MockPowerProvider {
    pub fn new() -> Arc<Self> {
        let (tx, _) = watch::channel(PowerStatus {
            battery_percentage: 75.5,
            is_charging: false,
            power_profile: PowerProfile::Balanced,
            has_battery: true,
        });

        Arc::new(Self { status_tx: tx })
    }

    pub fn simulate_change(&self, percentage: f64, charging: bool) {
        let mut status = self.status_tx.borrow().clone();
        status.battery_percentage = percentage;
        status.is_charging = charging;
        let _ = self.status_tx.send(status);
    }
}

#[async_trait]
impl PowerProvider for MockPowerProvider {
    async fn get_status(&self) -> Result<PowerStatus, PowerError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<PowerStream, PowerError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn suspend(&self) -> Result<(), PowerError> {
        info!("[mock-power] suspend");
        Ok(())
    }

    async fn power_off(&self) -> Result<(), PowerError> {
        info!("[mock-power] power_off");
        Ok(())
    }

    async fn reboot(&self) -> Result<(), PowerError> {
        info!("[mock-power] reboot");
        Ok(())
    }
}

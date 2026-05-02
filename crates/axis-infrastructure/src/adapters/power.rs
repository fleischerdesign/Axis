use axis_domain::models::power::PowerStatus;
use axis_domain::ports::power::{PowerProvider, PowerError, PowerStream};
use async_trait::async_trait;
use zbus::proxy;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;
use std::time::Duration;
use log::{info, warn};

#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/devices/DisplayDevice"
)]
trait UPowerDevice {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    #[zbus(property, name = "Type")]
    fn type_(&self) -> zbus::Result<u32>;
}

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    fn suspend(&self, interactive: bool) -> zbus::Result<()>;
    fn power_off(&self, interactive: bool) -> zbus::Result<()>;
    fn reboot(&self, interactive: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn sessions(&self) -> zbus::Result<Vec<(String, zbus::zvariant::ObjectPath<'_>)>>;
}

pub struct LogindPowerProvider {
    status_tx: watch::Sender<PowerStatus>,
    login1: Login1ManagerProxy<'static>,
}

impl LogindPowerProvider {
    pub async fn new() -> Result<Arc<Self>, PowerError> {
        let connection = zbus::Connection::system().await
            .map_err(|e| PowerError::ProviderError(e.to_string()))?;

        let proxy = UPowerDeviceProxy::new(&connection).await
            .map_err(|e| PowerError::ProviderError(e.to_string()))?;

        let login1 = Login1ManagerProxy::new(&connection).await
            .map_err(|e| PowerError::ProviderError(e.to_string()))?;

        let initial_status = Self::fetch_status(&proxy).await?;
        let (tx, _) = watch::channel(initial_status);
        let provider = Arc::new(Self { status_tx: tx, login1 });

        let provider_clone = provider.clone();
        tokio::spawn(async move {
            let mut attempt = 0u32;
            loop {
                match UPowerDeviceProxy::new(&connection).await {
                    Ok(proxy) => {
                        if attempt > 0 {
                            info!("[power] Reconnected to UPower");
                        }
                        attempt = 0;
                        let mut changes = proxy.receive_percentage_changed().await;
                        let mut state_changes = proxy.receive_state_changed().await;

                        loop {
                            let alive = tokio::select! {
                                Some(_) = changes.next() => true,
                                Some(_) = state_changes.next() => true,
                                else => false,
                            };
                            if !alive {
                                warn!("[power] UPower stream ended, reconnecting...");
                                break;
                            }
                            if let Ok(status) = Self::fetch_status(&proxy).await {
                                let _ = provider_clone.status_tx.send(status);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[power] Failed to connect to UPower: {e}, retrying...");
                    }
                }
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt.min(4)).min(30))).await;
            }
        });

        Ok(provider)
    }

    async fn fetch_status(proxy: &UPowerDeviceProxy<'_>) -> Result<PowerStatus, PowerError> {
        let percentage = proxy.percentage().await
            .map_err(|e| PowerError::ProviderError(e.to_string()))?;

        let state = proxy.state().await
            .map_err(|e| PowerError::ProviderError(e.to_string()))?;

        let is_charging = state == 1 || state == 4;

        let has_battery = proxy
            .type_()
            .await
            .map(|t| t == 2)
            .unwrap_or(false);

        Ok(PowerStatus {
            battery_percentage: percentage,
            is_charging,
            power_profile: "balanced".to_string(),
            has_battery,
        })
    }
}

#[async_trait]
impl PowerProvider for LogindPowerProvider {
    async fn get_status(&self) -> Result<PowerStatus, PowerError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<PowerStream, PowerError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn suspend(&self) -> Result<(), PowerError> {
        self.login1.suspend(true).await
            .map_err(|e| PowerError::ProviderError(e.to_string()))
    }

    async fn power_off(&self) -> Result<(), PowerError> {
        self.login1.power_off(true).await
            .map_err(|e| PowerError::ProviderError(e.to_string()))
    }

    async fn reboot(&self) -> Result<(), PowerError> {
        self.login1.reboot(true).await
            .map_err(|e| PowerError::ProviderError(e.to_string()))
    }
}

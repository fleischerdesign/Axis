use async_channel::{bounded, Sender};
use futures_util::StreamExt;
use std::time::Duration;
use zbus::{proxy, Connection};

use log::{error, info};

use super::Service;
use crate::store::ServiceStore;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PowerData {
    pub battery_percentage: f64,
    pub is_charging: bool,
    pub has_battery: bool,
}

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
    #[zbus(property)]
    fn is_present(&self) -> zbus::Result<bool>;
}

pub struct PowerService;

async fn fetch_power(upower: &UPowerDeviceProxy<'_>) -> PowerData {
    let percentage = upower.percentage().await.unwrap_or(0.0);
    let state = upower.state().await.unwrap_or(0);
    let is_present = upower.is_present().await.unwrap_or(false);

    PowerData {
        battery_percentage: (percentage * 10.0).round() / 10.0,
        is_charging: state == 1,
        has_battery: is_present && percentage > 0.0,
    }
}

impl Service for PowerService {
    type Data = PowerData;
    type Cmd = ();

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (tx, rx) = bounded(10);

        tokio::spawn(async move {
            // Retry connection setup with backoff
            let upower = loop {
                match Connection::system().await {
                    Ok(conn) => match UPowerDeviceProxy::new(&conn).await {
                        Ok(proxy) => break proxy,
                        Err(e) => error!("[power] Failed to create proxy: {e}"),
                    },
                    Err(e) => error!("[power] Failed to connect to D-Bus: {e}"),
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            };

            info!("[power] Connected to UPower");

            let mut pct_changed = upower.receive_percentage_changed().await;
            let mut state_changed = upower.receive_state_changed().await;
            let mut present_changed = upower.receive_is_present_changed().await;

            // Initial read
            let _ = tx.send(fetch_power(&upower).await).await;

            loop {
                tokio::select! {
                    Some(_) = pct_changed.next() => {},
                    Some(_) = state_changed.next() => {},
                    Some(_) = present_changed.next() => {},
                    else => break,
                }

                let _ = tx.send(fetch_power(&upower).await).await;
            }
        });

        let (dummy_tx, _) = bounded(1);
        (ServiceStore::new(rx, Default::default()), dummy_tx)
    }
}

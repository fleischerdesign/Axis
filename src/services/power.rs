use async_channel::{Receiver, bounded, Sender};
use std::thread;
use std::time::Duration;
use zbus::{proxy, Connection};

use super::Service;

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

impl Service for PowerService {
    type Data = PowerData;
    type Cmd = ();

    fn spawn() -> (Receiver<Self::Data>, Sender<Self::Cmd>) {
        let (tx, rx) = bounded(10);

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                // Retry connection setup with backoff
                let upower = loop {
                    match Connection::system().await {
                        Ok(conn) => match UPowerDeviceProxy::new(&conn).await {
                            Ok(proxy) => break proxy,
                            Err(e) => eprintln!("[PowerService] Failed to create proxy: {e}"),
                        },
                        Err(e) => eprintln!("[PowerService] Failed to connect to D-Bus: {e}"),
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                };

                loop {
                    let percentage = upower.percentage().await.unwrap_or(0.0);
                    let state = upower.state().await.unwrap_or(0); // 1 = Charging
                    let is_present = upower.is_present().await.unwrap_or(false);

                    let data = PowerData {
                        battery_percentage: (percentage * 10.0).round() / 10.0,
                        is_charging: state == 1,
                        has_battery: is_present && percentage > 0.0,
                    };

                    let _ = tx.send(data).await;
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            });
        });

        let (dummy_tx, _) = bounded(1);
        (rx, dummy_tx)
    }
}

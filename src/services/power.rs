use zbus::{Connection, proxy};
use async_channel::{Sender, Receiver, bounded};
use std::thread;

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

impl PowerService {
    pub fn spawn() -> (Receiver<PowerData>, Sender<PowerData>) {
        let (data_tx, data_rx) = bounded(100);
        let data_tx_return = data_tx.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let conn = Connection::system().await.unwrap();
                let upower = UPowerDeviceProxy::new(&conn).await.unwrap();

                loop {
                    let percentage = upower.percentage().await.unwrap_or(0.0);
                    let state = upower.state().await.unwrap_or(0); // 1=Charging
                    let is_present = upower.is_present().await.unwrap_or(false);

                    let new_data = PowerData {
                        battery_percentage: (percentage * 10.0).round() / 10.0,
                        is_charging: state == 1,
                        has_battery: is_present && percentage > 0.0,
                    };

                    let _ = data_tx.send(new_data).await;

                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            });
        });

        (data_rx, data_tx_return)
    }
}

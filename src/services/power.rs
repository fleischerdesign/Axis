use zbus::{Connection, proxy};
use futures_channel::mpsc;
use std::thread;

#[derive(Clone, Debug, Default)]
pub struct PowerData {
    pub battery_percentage: f64,
    pub is_charging: bool,
    pub has_battery: bool,
    pub active_profile: String,
    pub available_profiles: Vec<String>,
}

pub enum PowerCmd {
    SetProfile(String),
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

#[proxy(
    interface = "net.hadess.PowerProfiles",
    default_service = "net.hadess.PowerProfiles",
    default_path = "/net/hadess/PowerProfiles"
)]
trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<String>;
    #[zbus(property, name = "ActiveProfile")]
    fn set_active_profile(&self, profile: &str) -> zbus::Result<()>;
    #[zbus(property)]
    fn profiles(&self) -> zbus::Result<Vec<std::collections::HashMap<String, String>>>;
}

pub struct PowerService;

impl PowerService {
    pub fn spawn() -> (mpsc::UnboundedReceiver<PowerData>, mpsc::UnboundedSender<PowerCmd>) {
        let (data_tx, data_rx) = mpsc::unbounded::<PowerData>();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded::<PowerCmd>();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let conn = Connection::system().await.unwrap();
                let upower = UPowerDeviceProxy::new(&conn).await.unwrap();
                let profiles = PowerProfilesProxy::new(&conn).await.unwrap();

                loop {
                    // 1. Akku Daten
                    let percentage = upower.percentage().await.unwrap_or(0.0);
                    let state = upower.state().await.unwrap_or(0); // 1=Charging
                    let is_present = upower.is_present().await.unwrap_or(false);

                    // 2. Profile Daten
                    let active_profile = profiles.active_profile().await.unwrap_or_else(|_| "balanced".to_string());
                    let raw_profiles = profiles.profiles().await.unwrap_or_default();
                    let available_profiles: Vec<String> = raw_profiles.into_iter()
                        .filter_map(|p| p.get("Profile").cloned())
                        .collect();

                    let _ = data_tx.unbounded_send(PowerData {
                        battery_percentage: percentage,
                        is_charging: state == 1,
                        has_battery: is_present && percentage > 0.0,
                        active_profile,
                        available_profiles,
                    });

                    // 3. Befehle verarbeiten
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        match cmd {
                            PowerCmd::SetProfile(p) => {
                                let _ = profiles.set_active_profile(&p).await;
                            }
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            });
        });

        (data_rx, cmd_tx)
    }
}

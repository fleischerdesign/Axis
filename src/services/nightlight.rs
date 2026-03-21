use super::Service;
use crate::store::ServiceStore;
use async_channel::{bounded, Sender};
use log::{error, info, warn};
use std::process::{Child, Command, Stdio};
use std::thread;

#[derive(Clone, Debug, PartialEq)]
pub struct NightlightData {
    pub enabled: bool,
    pub available: bool,
    pub temp_day: u32,
    pub temp_night: u32,
    pub sunrise: String,
    pub sunset: String,
    pub latitude: String,
    pub longitude: String,
}

impl Default for NightlightData {
    fn default() -> Self {
        Self {
            enabled: false,
            available: false,
            temp_day: 6500,
            temp_night: 4500,
            sunrise: "07:00".to_string(),
            sunset: "20:00".to_string(),
            latitude: "".to_string(),
            longitude: "".to_string(),
        }
    }
}

pub enum NightlightCmd {
    Toggle(bool),
    SetTempDay(u32),
    SetTempNight(u32),
    SetSchedule(String, String), // sunrise, sunset
    SetLocation(String, String), // lat, long
}

pub struct NightlightService;

impl Service for NightlightService {
    type Data = NightlightData;
    type Cmd = NightlightCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(100);
        let (cmd_tx, cmd_rx) = bounded(100);

        thread::spawn(move || {
            let mut wlsunset_child: Option<Child> = None;
            let mut data = NightlightData {
                available: Self::check_available(),
                ..Default::default()
            };

            if data.available {
                info!("[nightlight] wlsunset available");
            } else {
                info!("[nightlight] wlsunset not available");
            }

            loop {
                // Handle commands
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        NightlightCmd::Toggle(on) => {
                            info!("[nightlight] {}", if on { "enabled" } else { "disabled" });
                            if on {
                                if !data.enabled {
                                    if let Some(child) = Self::start_wlsunset(&data) {
                                        wlsunset_child = Some(child);
                                        data.enabled = true;
                                    }
                                }
                            } else if data.enabled {
                                if let Some(mut child) = wlsunset_child.take() {
                                    Self::stop_wlsunset(&mut child);
                                    data.enabled = false;
                                }
                            }
                        }
                        NightlightCmd::SetTempDay(kelvin) => {
                            data.temp_day = kelvin;
                            if data.enabled {
                                if let Some(mut child) = wlsunset_child.take() {
                                    Self::stop_wlsunset(&mut child);
                                }
                                if let Some(child) = Self::start_wlsunset(&data) {
                                    wlsunset_child = Some(child);
                                }
                            }
                        }
                        NightlightCmd::SetTempNight(kelvin) => {
                            data.temp_night = kelvin;
                            if data.enabled {
                                if let Some(mut child) = wlsunset_child.take() {
                                    Self::stop_wlsunset(&mut child);
                                }
                                if let Some(child) = Self::start_wlsunset(&data) {
                                    wlsunset_child = Some(child);
                                }
                            }
                        }
                        NightlightCmd::SetSchedule(sunrise, sunset) => {
                            data.sunrise = sunrise;
                            data.sunset = sunset;
                            data.latitude = "".to_string();
                            data.longitude = "".to_string();
                            if data.enabled {
                                if let Some(mut child) = wlsunset_child.take() {
                                    Self::stop_wlsunset(&mut child);
                                }
                                if let Some(child) = Self::start_wlsunset(&data) {
                                    wlsunset_child = Some(child);
                                }
                            }
                        }
                        NightlightCmd::SetLocation(lat, long) => {
                            data.latitude = lat;
                            data.longitude = long;
                            data.sunrise = "".to_string();
                            data.sunset = "".to_string();
                            if data.enabled {
                                if let Some(mut child) = wlsunset_child.take() {
                                    Self::stop_wlsunset(&mut child);
                                }
                                if let Some(child) = Self::start_wlsunset(&data) {
                                    wlsunset_child = Some(child);
                                }
                            }
                        }
                    }

                    let _ = data_tx.send_blocking(data.clone());
                }

                // If wlsunset crashed, reset state
                if let Some(child) = &mut wlsunset_child {
                    match child.try_wait() {
                        Ok(Some(_)) => {
                            warn!("[nightlight] wlsunset exited");
                            wlsunset_child = None;
                            data.enabled = false;
                            let _ = data_tx.send_blocking(data.clone());
                        }
                        Ok(None) => {} // Still running
                        Err(e) => {
                            error!("[nightlight] Error checking wlsunset: {e}");
                            wlsunset_child = None;
                            data.enabled = false;
                            let _ = data_tx.send_blocking(data.clone());
                        }
                    }
                }

                thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        (
            ServiceStore::new(data_rx, NightlightService::read_initial()),
            cmd_tx,
        )
    }
}

impl NightlightService {
    fn check_available() -> bool {
        Command::new("which")
            .arg("wlsunset")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn read_initial() -> NightlightData {
        NightlightData {
            available: Self::check_available(),
            ..Default::default()
        }
    }

    fn start_wlsunset(data: &NightlightData) -> Option<Child> {
        let mut cmd = Command::new("wlsunset");
        cmd.arg("-t").arg(data.temp_night.to_string());
        cmd.arg("-T").arg(data.temp_day.to_string());

        if !data.latitude.is_empty() && !data.longitude.is_empty() {
            cmd.arg("-l").arg(&data.latitude);
            cmd.arg("-L").arg(&data.longitude);
        } else if !data.sunrise.is_empty() && !data.sunset.is_empty() {
            cmd.arg("-S").arg(&data.sunrise);
            cmd.arg("-s").arg(&data.sunset);
        }

        match cmd.spawn() {
            Ok(child) => Some(child),
            Err(e) => {
                error!("[nightlight] Failed to start wlsunset: {e}");
                None
            }
        }
    }

    fn stop_wlsunset(child: &mut Child) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

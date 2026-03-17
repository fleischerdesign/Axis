use async_channel::{bounded, Receiver, Sender};
use std::process::{Command, Stdio};
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

impl NightlightService {
    pub fn spawn() -> (Receiver<NightlightData>, Sender<NightlightCmd>) {
        let (data_tx, data_rx) = bounded(100);
        let (cmd_tx, cmd_rx) = bounded(100);

        thread::spawn(move || {
            let mut wlsunset_pid = None;
            let mut data = NightlightData {
                available: Self::check_available(),
                ..Default::default()
            };

            loop {
                // Handle commands
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        NightlightCmd::Toggle(on) => {
                            if on {
                                if !data.enabled {
                                    if let Some(pid) = Self::start_wlsunset(&data) {
                                        wlsunset_pid = Some(pid);
                                        data.enabled = true;
                                    }
                                }
                            } else if data.enabled {
                                if let Some(pid) = wlsunset_pid.take() {
                                    Self::stop_wlsunset(pid);
                                    data.enabled = false;
                                }
                            }
                        }
                        NightlightCmd::SetTempDay(kelvin) => {
                            data.temp_day = kelvin;
                            if data.enabled {
                                if let Some(pid) = wlsunset_pid.take() {
                                    Self::stop_wlsunset(pid);
                                }
                                if let Some(pid) = Self::start_wlsunset(&data) {
                                    wlsunset_pid = Some(pid);
                                }
                            }
                        }
                        NightlightCmd::SetTempNight(kelvin) => {
                            data.temp_night = kelvin;
                            if data.enabled {
                                if let Some(pid) = wlsunset_pid.take() {
                                    Self::stop_wlsunset(pid);
                                }
                                if let Some(pid) = Self::start_wlsunset(&data) {
                                    wlsunset_pid = Some(pid);
                                }
                            }
                        }
                        NightlightCmd::SetSchedule(sunrise, sunset) => {
                            data.sunrise = sunrise;
                            data.sunset = sunset;
                            data.latitude = "".to_string();
                            data.longitude = "".to_string();
                            if data.enabled {
                                if let Some(pid) = wlsunset_pid.take() {
                                    Self::stop_wlsunset(pid);
                                }
                                if let Some(pid) = Self::start_wlsunset(&data) {
                                    wlsunset_pid = Some(pid);
                                }
                            }
                        }
                        NightlightCmd::SetLocation(lat, long) => {
                            data.latitude = lat;
                            data.longitude = long;
                            data.sunrise = "".to_string();
                            data.sunset = "".to_string();
                            if data.enabled {
                                if let Some(pid) = wlsunset_pid.take() {
                                    Self::stop_wlsunset(pid);
                                }
                                if let Some(pid) = Self::start_wlsunset(&data) {
                                    wlsunset_pid = Some(pid);
                                }
                            }
                        }
                    }

                    let _ = data_tx.send_blocking(data.clone());
                }

                // If wlsunset crashed, reset state
                if let Some(pid) = wlsunset_pid {
                    if !Self::is_pid_alive(pid) {
                        wlsunset_pid = None;
                        data.enabled = false;
                        let _ = data_tx.send_blocking(data.clone());
                    }
                }

                thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        (data_rx, cmd_tx)
    }

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

    fn start_wlsunset(data: &NightlightData) -> Option<u32> {
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

        let child = cmd.spawn().ok()?;

        Some(child.id())
    }

    fn stop_wlsunset(pid: u32) {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }

    fn is_pid_alive(pid: u32) -> bool {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

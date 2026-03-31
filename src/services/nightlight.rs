use super::{Service, ServiceConfig};
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
                // Block until a command arrives (no CPU-burning poll)
                match cmd_rx.recv_blocking() {
                    Ok(cmd) => {
                        Self::apply_cmd(cmd, &mut data, &mut wlsunset_child);
                        let _ = data_tx.send_blocking(data.clone());
                    }
                    Err(_) => break,
                }

                // Drain any additional queued commands before checking process
                while let Ok(cmd) = cmd_rx.try_recv() {
                    Self::apply_cmd(cmd, &mut data, &mut wlsunset_child);
                    let _ = data_tx.send_blocking(data.clone());
                }

                // Check if wlsunset crashed
                Self::check_crashed(&mut wlsunset_child, &mut data, &data_tx);
            }
        });

        (
            ServiceStore::new(data_rx, NightlightService::read_initial()),
            cmd_tx,
        )
    }
}

impl ServiceConfig for NightlightService {
    fn get_enabled(data: &NightlightData) -> bool { data.enabled }
    fn cmd_set_enabled(on: bool) -> NightlightCmd { NightlightCmd::Toggle(on) }
}

impl NightlightService {
    fn apply_cmd(cmd: NightlightCmd, data: &mut NightlightData, child: &mut Option<Child>) {
        match cmd {
            NightlightCmd::Toggle(on) => {
                info!("[nightlight] {}", if on { "enabled" } else { "disabled" });
                if on {
                    if !data.enabled {
                        if let Some(c) = Self::start_wlsunset(data) {
                            *child = Some(c);
                            data.enabled = true;
                        }
                    }
                } else if data.enabled {
                    Self::stop_child(child);
                    data.enabled = false;
                }
            }
            NightlightCmd::SetTempDay(k) => {
                data.temp_day = k;
                Self::restart_if_enabled(child, data);
            }
            NightlightCmd::SetTempNight(k) => {
                data.temp_night = k;
                Self::restart_if_enabled(child, data);
            }
            NightlightCmd::SetSchedule(sr, ss) => {
                data.sunrise = sr;
                data.sunset = ss;
                data.latitude.clear();
                data.longitude.clear();
                Self::restart_if_enabled(child, data);
            }
            NightlightCmd::SetLocation(la, lo) => {
                data.latitude = la;
                data.longitude = lo;
                data.sunrise.clear();
                data.sunset.clear();
                Self::restart_if_enabled(child, data);
            }
        }
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

    fn stop_child(child: &mut Option<Child>) {
        if let Some(mut c) = child.take() {
            Self::stop_wlsunset(&mut c);
        }
    }

    fn restart_if_enabled(child: &mut Option<Child>, data: &NightlightData) {
        if data.enabled {
            Self::stop_child(child);
            *child = Self::start_wlsunset(data);
        }
    }

    fn check_crashed(
        child: &mut Option<Child>,
        data: &mut NightlightData,
        tx: &Sender<NightlightData>,
    ) {
        if let Some(c) = child {
            match c.try_wait() {
                Ok(Some(_)) => {
                    warn!("[nightlight] wlsunset exited");
                    *child = None;
                    data.enabled = false;
                    let _ = tx.send_blocking(data.clone());
                }
                Ok(None) => {}
                Err(e) => {
                    error!("[nightlight] Error checking wlsunset: {e}");
                    *child = None;
                    data.enabled = false;
                    let _ = tx.send_blocking(data.clone());
                }
            }
        }
    }
}

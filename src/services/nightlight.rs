use async_channel::{bounded, Receiver, Sender};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NightlightData {
    pub enabled: bool,
    pub available: bool,
    pub temperature: u32,
}

pub enum NightlightCmd {
    Toggle(bool),
    SetTemperature(u32),
}

pub struct NightlightService;

impl NightlightService {
    pub fn spawn() -> (Receiver<NightlightData>, Sender<NightlightCmd>) {
        let (data_tx, data_rx) = bounded(100);
        let (cmd_tx, cmd_rx) = bounded(100);

        thread::spawn(move || {
            let mut wlsunset_pid = None;
            let mut enabled = false;
            let mut temperature = 4500;
            let available = Self::check_available();

            loop {
                // Handle commands
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        NightlightCmd::Toggle(on) => {
                            if on {
                                if !enabled {
                                    if let Some(pid) = Self::start_wlsunset(temperature) {
                                        wlsunset_pid = Some(pid);
                                        enabled = true;
                                    }
                                }
                            } else if enabled {
                                if let Some(pid) = wlsunset_pid.take() {
                                    Self::stop_wlsunset(pid);
                                    enabled = false;
                                }
                            }
                        }
                        NightlightCmd::SetTemperature(kelvin) => {
                            temperature = kelvin;
                            if enabled {
                                if let Some(pid) = wlsunset_pid.take() {
                                    Self::stop_wlsunset(pid);
                                }
                                if let Some(pid) = Self::start_wlsunset(temperature) {
                                    wlsunset_pid = Some(pid);
                                }
                            }
                        }
                    }

                    let _ = data_tx.send_blocking(NightlightData {
                        enabled,
                        available,
                        temperature,
                    });
                }

                // If wlsunset crashed, reset state
                if let Some(pid) = wlsunset_pid {
                    if !Self::is_pid_alive(pid) {
                        wlsunset_pid = None;
                        enabled = false;
                        let _ = data_tx.send_blocking(NightlightData {
                            enabled,
                            available,
                            temperature,
                        });
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
            enabled: false,
            available: Self::check_available(),
            temperature: 4500,
        }
    }

    fn start_wlsunset(temperature: u32) -> Option<u32> {
        let high = temperature + 1000;
        let child = Command::new("wlsunset")
            .arg("-S")
            .arg("00:00")
            .arg("-s")
            .arg("23:59")
            .arg("-T")
            .arg(high.to_string())
            .arg("-t")
            .arg(temperature.to_string())
            .spawn()
            .ok()?;

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

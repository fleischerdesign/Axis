use async_channel::{bounded, Receiver, Sender};
use brightness::blocking::Brightness;
use inotify::{Inotify, WatchMask};
use std::fs;
use std::path::PathBuf;
use std::thread;

const BACKLIGHT_DIR: &str = "/sys/class/backlight";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BacklightData {
    pub percentage: f64,
    pub has_backlight: bool,
    pub initialized: bool,
}

pub enum BacklightCmd {
    SetBrightness(f64),
}

pub struct BacklightService;

impl BacklightService {
    pub fn spawn() -> (Receiver<BacklightData>, Sender<BacklightCmd>) {
        let (data_tx, data_rx) = bounded(1);
        let (cmd_tx, cmd_rx) = bounded(16);

        thread::spawn(move || {
            let device_path = match find_backlight_device() {
                Some(p) => p,
                None => {
                    let _ = data_tx.send_blocking(BacklightData::default());
                    // Kein Device: trotzdem auf Commands warten (kann später erscheinen)
                    loop {
                        let _ = cmd_rx.recv_blocking();
                    }
                }
            };

            let actual_path = device_path.join("actual_brightness");
            let max_path = device_path.join("max_brightness");

            let max = read_sysfs_u32(&max_path).unwrap_or(1);

            // Initialen Zustand senden
            if let Some(actual) = read_sysfs_u32(&actual_path) {
                let _ = data_tx.send_blocking(BacklightData {
                    percentage: actual as f64 / max as f64 * 100.0,
                    has_backlight: true,
                    initialized: true,
                });
            }

            // inotify auf actual_brightness registrieren
            let inotify = match Inotify::init() {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("[BacklightService] inotify init failed: {e}");
                    // Fallback: Polling
                    polling_loop(&actual_path, max, &data_tx, &cmd_rx);
                    return;
                }
            };

            let mut watcher = inotify;

            // Watch auf das Device-Verzeichnis (actual_brightness wird darin aktualisiert)
            if watcher
                .watches()
                .add(&device_path, WatchMask::MODIFY)
                .is_err()
            {
                // Fallback: Polling
                polling_loop(&actual_path, max, &data_tx, &cmd_rx);
                return;
            }

            let mut buf = [0u8; 4096];

            // Command-Thread: Brightness setzen über brightness Crate (D-Bus)
            thread::spawn(move || {
                let brightness_devices = brightness::blocking::brightness_devices();
                for dev in brightness_devices.flatten() {
                    loop {
                        let cmd = match cmd_rx.recv_blocking() {
                            Ok(c) => c,
                            Err(_) => return,
                        };
                        let BacklightCmd::SetBrightness(pct) = cmd;
                        let _ = dev.set(pct as u32);
                    }
                }
            });

            // Event-Loop: auf inotify Events warten
            let mut has_events = false;

            loop {
                let events = match watcher.read_events_blocking(&mut buf) {
                    Ok(e) => e,
                    Err(_) => break,
                };

                let mut should_update = false;
                for event in events {
                    has_events = true;
                    if let Some(name) = event.name {
                        if name == "actual_brightness" {
                            should_update = true;
                        }
                    }
                }

                // Fallback: bei MODIFY ohne spezifischen Namen trotzdem aktualisieren
                if !should_update && has_events {
                    should_update = true;
                }
                has_events = false;

                if should_update {
                    if let Some(actual) = read_sysfs_u32(&actual_path) {
                        let pct = actual as f64 / max as f64 * 100.0;
                        let _ = data_tx.send_blocking(BacklightData {
                            percentage: pct,
                            has_backlight: true,
                            initialized: true,
                        });
                    }
                }
            }
        });

        (data_rx, cmd_tx)
    }

    pub fn read_initial() -> BacklightData {
        if let Some(path) = find_backlight_device() {
            let max = read_sysfs_u32(&path.join("max_brightness")).unwrap_or(1);
            if let Some(actual) = read_sysfs_u32(&path.join("actual_brightness")) {
                return BacklightData {
                    percentage: actual as f64 / max as f64 * 100.0,
                    has_backlight: true,
                    initialized: true,
                };
            }
        }
        BacklightData::default()
    }
}

fn find_backlight_device() -> Option<PathBuf> {
    let entries = fs::read_dir(BACKLIGHT_DIR).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.join("actual_brightness").exists() && path.join("max_brightness").exists() {
            return Some(path);
        }
    }
    None
}

fn read_sysfs_u32(path: &PathBuf) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse::<u32>().ok()
}

fn polling_loop(
    actual_path: &PathBuf,
    max: u32,
    data_tx: &Sender<BacklightData>,
    cmd_rx: &Receiver<BacklightCmd>,
) {
    let mut last_pct: Option<f64> = None;

    loop {
        if let Some(actual) = read_sysfs_u32(actual_path) {
            let pct = actual as f64 / max as f64 * 100.0;
            if last_pct.map(|l| (l - pct).abs() > 0.01).unwrap_or(true) {
                last_pct = Some(pct);
                let _ = data_tx.send_blocking(BacklightData {
                    percentage: pct,
                    has_backlight: true,
                    initialized: true,
                });
            }
        }

        while let Ok(cmd) = cmd_rx.try_recv() {
            let BacklightCmd::SetBrightness(pct) = cmd;
            let brightness_devices = brightness::blocking::brightness_devices();
            for dev in brightness_devices.flatten() {
                let _ = dev.set(pct as u32);
            }
        }

        thread::sleep(std::time::Duration::from_millis(100));
    }
}

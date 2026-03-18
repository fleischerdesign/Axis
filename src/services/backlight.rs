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
                    return;
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

            // inotify auf Device-Verzeichnis registrieren
            let mut watcher = Inotify::init().expect("Failed to init inotify");
            watcher
                .watches()
                .add(&device_path, WatchMask::MODIFY)
                .expect("Failed to watch backlight device");

            // Command-Thread: Brightness setzen über brightness Crate (D-Bus)
            thread::spawn(move || {
                for dev in brightness::blocking::brightness_devices().flatten() {
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
            let mut buf = [0u8; 4096];
            loop {
                let events = match watcher.read_events_blocking(&mut buf) {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("[BacklightService] inotify read failed: {e}");
                        break;
                    }
                };

                let mut changed = false;
                for event in events {
                    if let Some(name) = event.name {
                        if name == "actual_brightness" {
                            changed = true;
                        }
                    } else {
                        // MODIFY ohne spezifischen Namen → immer aktualisieren
                        changed = true;
                    }
                }

                if changed {
                    if let Some(actual) = read_sysfs_u32(&actual_path) {
                        let _ = data_tx.send_blocking(BacklightData {
                            percentage: actual as f64 / max as f64 * 100.0,
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
    for entry in fs::read_dir(BACKLIGHT_DIR).ok()?.flatten() {
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

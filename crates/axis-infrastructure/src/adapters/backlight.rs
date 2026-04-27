use axis_domain::models::brightness::BrightnessStatus;
use axis_domain::ports::brightness::{BrightnessProvider, BrightnessError, BrightnessStream};
use async_trait::async_trait;
use tokio::sync::{watch, mpsc};
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use inotify::{Inotify, WatchMask};
use brightness::blocking::Brightness;
use log::warn;

const BACKLIGHT_DIR: &str = "/sys/class/backlight";

pub struct SysfsBrightnessProvider {
    status_tx: watch::Sender<BrightnessStatus>,
    cmd_tx: mpsc::Sender<f64>,
}

impl SysfsBrightnessProvider {
    pub async fn new() -> Result<Arc<Self>, BrightnessError> {
        let device_path = Self::find_device().ok_or_else(|| BrightnessError::ProviderError("No backlight device found".into()))?;
        
        // Read initial value immediately
        let actual = Self::read_u32(&device_path.join("actual_brightness")).unwrap_or(0);
        let max = Self::read_u32(&device_path.join("max_brightness")).unwrap_or(1);
        let initial_pct = (actual as f64 / max as f64) * 100.0;

        let (status_tx, _) = watch::channel(BrightnessStatus {
            percentage: initial_pct,
            has_backlight: true,
        });

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<f64>(32);
        let status_tx_clone = status_tx.clone();

        std::thread::spawn(move || {
            let actual_path = device_path.join("actual_brightness");
            let max_path = device_path.join("max_brightness");
            let max = Self::read_u32(&max_path).unwrap_or(1);

            loop {
                if let Ok(mut inotify) = Inotify::init() {
                    let _ = inotify.watches().add(&device_path, WatchMask::MODIFY);
                    let mut buf = [0u8; 4096];

                    loop {
                        if let Some(actual) = Self::read_u32(&actual_path) {
                            let pct = (actual as f64 / max as f64) * 100.0;
                            let _ = status_tx_clone.send(BrightnessStatus {
                                percentage: pct,
                                has_backlight: true,
                            });
                        }
                        if inotify.read_events_blocking(&mut buf).is_err() {
                            warn!("[backlight] inotify error, reinitializing...");
                            break;
                        }
                    }
                } else {
                    warn!("[backlight] Failed to init inotify, retrying in 5s...");
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        });

        std::thread::spawn(move || {
            if let Some(device) = brightness::blocking::brightness_devices().flatten().next() {
                while let Some(pct) = cmd_rx.blocking_recv() {
                    let mut latest = pct;
                    while let Ok(newer) = cmd_rx.try_recv() {
                        latest = newer;
                    }
                    let _ = device.set(latest as u32);
                }
            }
        });

        Ok(Arc::new(Self { status_tx, cmd_tx }))
    }

    fn find_device() -> Option<PathBuf> {
        fs::read_dir(BACKLIGHT_DIR).ok()?.flatten().find_map(|entry| {
            let path = entry.path();
            if path.join("actual_brightness").exists() { Some(path) } else { None }
        })
    }

    fn read_u32(path: &PathBuf) -> Option<u32> {
        fs::read_to_string(path).ok()?.trim().parse::<u32>().ok()
    }
}

#[async_trait]
impl BrightnessProvider for SysfsBrightnessProvider {
    async fn get_status(&self) -> Result<BrightnessStatus, BrightnessError> {
        Ok(self.status_tx.borrow().clone())
    }
    async fn subscribe(&self) -> Result<BrightnessStream, BrightnessError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }
    async fn set_brightness(&self, percentage: f64) -> Result<(), BrightnessError> {
        let _ = self.cmd_tx.send(percentage).await;
        Ok(())
    }
}

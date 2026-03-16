use async_channel::{bounded, Receiver, Sender};
use brightness::blocking::Brightness;
use std::thread;

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
        let (data_tx, data_rx) = bounded(100);
        let (cmd_tx, cmd_rx) = bounded(100);

        thread::spawn(move || {
            let mut last_percentage = None;

            loop {
                Self::push_update(&data_tx, &mut last_percentage);

                while let Ok(cmd) = cmd_rx.try_recv() {
                    let BacklightCmd::SetBrightness(pct) = cmd;
                    let devices = brightness::blocking::brightness_devices();
                    for device in devices.flatten() {
                        let _ = device.set(pct as u32);
                    }
                }

                thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        (data_rx, cmd_tx)
    }

    pub fn read_initial() -> BacklightData {
        use brightness::blocking::Brightness;
        let devices = brightness::blocking::brightness_devices();
        for device in devices.flatten() {
            if let Ok(pct) = device.get() {
                return BacklightData {
                    percentage: pct as f64,
                    has_backlight: true,
                    initialized: true,
                };
            }
        }
        BacklightData::default()
    }

    fn push_update(tx: &Sender<BacklightData>, last: &mut Option<f64>) {
        let devices = brightness::blocking::brightness_devices();
        let mut percentage: Option<u32> = None;
        let mut has_backlight = false;

        for device in devices.flatten() {
            has_backlight = true;
            if let Ok(pct) = device.get() {
                percentage = Some(pct);
                break;
            }
        }

        let pct = percentage.unwrap_or(0) as f64;
        let new_data = BacklightData {
            percentage: pct,
            has_backlight,
            initialized: true,
        };

        if last.map(|p| (p - pct).abs() > 0.5).unwrap_or(true) {
            *last = Some(pct);
            let _ = tx.send_blocking(new_data);
        }
    }
}

use async_channel::{bounded, Receiver, Sender};
use libpulse_binding::volume::Volume;
use pulsectl::controllers::DeviceControl;
use pulsectl::controllers::SinkController;
use std::thread;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioData {
    pub volume: f64,
    pub is_muted: bool,
}

pub enum AudioCmd {
    SetVolume(f64),
    #[allow(dead_code)]
    SetMute(bool),
}

pub struct AudioService;

impl AudioService {
    pub fn spawn() -> (Receiver<AudioData>, Sender<AudioCmd>) {
        let (data_tx, data_rx) = bounded(100);
        let (cmd_tx, cmd_rx) = bounded(100);

        thread::spawn(move || {
            let mut handler = match SinkController::create() {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("[AudioService] Failed to create SinkController: {:?}", e);
                    return;
                }
            };

            let mut last_data = AudioData::default();

            loop {
                if let Some(new_data) = Self::get_current_data(&mut handler) {
                    if new_data != last_data {
                        let _ = data_tx.send_blocking(new_data.clone());
                        last_data = new_data;
                    }
                }

                // Befehle verarbeiten
                while let Ok(cmd) = cmd_rx.try_recv() {
                    if let Ok(sink) = handler.get_default_device() {
                        match cmd {
                            AudioCmd::SetVolume(new_vol) => {
                                let pulse_vol = Volume(
                                    ((new_vol * Volume::NORMAL.0 as f64) as u32)
                                        .min(Volume::NORMAL.0 * 2),
                                );
                                let mut cv = sink.volume.clone();
                                let channels = cv.len();
                                cv.set(channels, pulse_vol);
                                if let Some(name) = &sink.name {
                                    let _ = handler.set_device_volume_by_name(name, &cv);
                                }
                            }
                            AudioCmd::SetMute(mute) => {
                                if let Some(name) = &sink.name {
                                    let _ = handler.set_device_mute_by_name(name, mute);
                                }
                            }
                        }
                    }
                }

                thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        (data_rx, cmd_tx)
    }

    pub fn read_initial() -> AudioData {
        if let Ok(mut handler) = SinkController::create() {
            if let Some(data) = Self::get_current_data(&mut handler) {
                return data;
            }
        }
        AudioData::default()
    }

    fn get_current_data(handler: &mut SinkController) -> Option<AudioData> {
        if let Ok(sink) = handler.get_default_device() {
            let vol_raw = sink.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
            return Some(AudioData {
                volume: (vol_raw * 100.0).round() / 100.0,
                is_muted: sink.mute,
            });
        }
        None
    }
}

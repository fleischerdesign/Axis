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

            // Initialer fetch
            Self::push_update(&mut handler, &data_tx);

            loop {
                // Wir nutzen hier immer noch einen kleinen Sleep, da pulsectl-rs
                // kein einfaches async-Event-Interface für Sink-Changes bietet,
                // aber wir deduplizieren die Sends massiv.

                Self::push_update(&mut handler, &data_tx);

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

                // Höherer Sleep-Intervall, da wir nur bei Änderungen senden
                thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        (data_rx, cmd_tx)
    }

    fn push_update(handler: &mut SinkController, tx: &Sender<AudioData>) {
        if let Ok(sink) = handler.get_default_device() {
            let vol_raw = sink.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
            let new_data = AudioData {
                volume: (vol_raw * 100.0).round() / 100.0,
                is_muted: sink.mute,
            };

            let _ = tx.send_blocking(new_data);
        }
    }
}

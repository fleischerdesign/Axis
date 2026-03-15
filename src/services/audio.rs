use pulsectl::controllers::SinkController;
use pulsectl::controllers::DeviceControl;
use libpulse_binding::volume::Volume;
use futures_channel::mpsc;
use std::thread;

#[derive(Clone, Debug)]
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
    pub fn spawn() -> (
        mpsc::UnboundedReceiver<AudioData>,
        mpsc::UnboundedSender<AudioCmd>,
    ) {
        let (data_tx, data_rx) = mpsc::unbounded::<AudioData>();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded::<AudioCmd>();

        thread::spawn(move || {
            let mut handler = match SinkController::create() {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("[AudioService] Failed to create SinkController: {:?}", e);
                    return;
                }
            };

            loop {
                if let Ok(sink) = handler.get_default_device() {
                    let vol_raw = sink.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
                    
                    let _ = data_tx.unbounded_send(AudioData {
                        volume: vol_raw,
                        is_muted: sink.mute,
                    });

                    while let Ok(cmd) = cmd_rx.try_recv() {
                        match cmd {
                            AudioCmd::SetVolume(new_vol) => {
                                let pulse_vol = Volume(((new_vol * Volume::NORMAL.0 as f64) as u32).min(Volume::NORMAL.0 * 2));
                                let mut cv = sink.volume.clone();
                                let channels = cv.len();
                                cv.set(channels, pulse_vol);
                                
                                if let Some(name) = &sink.name {
                                    let _ = handler.set_device_volume_by_name(name, &cv);
                                }
                            },
                            AudioCmd::SetMute(mute) => {
                                if let Some(name) = &sink.name {
                                    let _ = handler.set_device_mute_by_name(name, mute);
                                }
                            }
                        }
                    }
                }

                thread::sleep(std::time::Duration::from_millis(150));
            }
        });

        (data_rx, cmd_tx)
    }
}

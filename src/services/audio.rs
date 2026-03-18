use async_channel::{bounded, Receiver, Sender};
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::SinkInfo;
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet, Operation};
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::mainloop::threaded::Mainloop;
use libpulse_binding::volume::{ChannelVolumes, Volume};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioData {
    pub volume: f64,
    pub is_muted: bool,
}

pub enum AudioCmd {
    SetVolume(f64),
    SetMute(bool),
}

pub struct AudioService;

impl AudioService {
    pub fn spawn() -> (Receiver<AudioData>, Sender<AudioCmd>) {
        let (data_tx, data_rx) = bounded(1);
        let (cmd_tx, cmd_rx) = bounded(16);

        thread::spawn(move || {
            let mainloop = match Mainloop::new() {
                Some(ml) => Rc::new(RefCell::new(ml)),
                None => {
                    eprintln!("[AudioService] Failed to create PulseAudio mainloop");
                    return;
                }
            };

            let context = match Context::new(&*mainloop.borrow(), "carp-audio") {
                Some(ctx) => Rc::new(RefCell::new(ctx)),
                None => {
                    eprintln!("[AudioService] Failed to create PulseAudio context");
                    return;
                }
            };

            // State-Callback: auf Ready warten
            {
                let ml_ref = Rc::clone(&mainloop);
                let ctx_ref = Rc::clone(&context);
                context
                    .borrow_mut()
                    .set_state_callback(Some(Box::new(move || {
                        let state = unsafe { (*ctx_ref.as_ptr()).get_state() };
                        match state {
                            ContextState::Ready
                            | ContextState::Failed
                            | ContextState::Terminated => unsafe {
                                (*ml_ref.as_ptr()).signal(false);
                            },
                            _ => {}
                        }
                    })));
            }

            context
                .borrow_mut()
                .connect(None, ContextFlagSet::NOFLAGS, None)
                .expect("Failed to connect PulseAudio context");

            mainloop.borrow_mut().lock();
            mainloop
                .borrow_mut()
                .start()
                .expect("Failed to start PulseAudio mainloop");

            // Warten bis Context Ready oder Failed
            loop {
                match context.borrow().get_state() {
                    ContextState::Ready => break,
                    ContextState::Failed | ContextState::Terminated => {
                        eprintln!("[AudioService] PulseAudio context failed");
                        mainloop.borrow_mut().unlock();
                        mainloop.borrow_mut().stop();
                        return;
                    }
                    _ => {
                        mainloop.borrow_mut().wait();
                    }
                }
            }

            // Subscribe für Sink-Events registrieren
            let ctx_ref = Rc::clone(&context);
            let data_tx2 = data_tx.clone();
            context.borrow_mut().set_subscribe_callback(Some(Box::new(
                move |facility: Option<Facility>, _op: Option<Operation>, _index: u32| {
                    if facility == Some(Facility::Sink) {
                        Self::fetch_and_send(&ctx_ref, &data_tx2);
                    }
                },
            )));

            context
                .borrow_mut()
                .subscribe(InterestMaskSet::SINK, |_| {});

            // Initiales Daten senden
            Self::fetch_and_send(&context, &data_tx);

            mainloop.borrow_mut().unlock();

            // Command-Loop: wartet auf Commands und verarbeitet sie
            loop {
                let cmd = match cmd_rx.recv_blocking() {
                    Ok(cmd) => cmd,
                    Err(_) => break,
                };

                mainloop.borrow_mut().lock();

                match cmd {
                    AudioCmd::SetVolume(new_vol) => {
                        let pulse_vol = Volume(
                            ((new_vol * Volume::NORMAL.0 as f64) as u32).min(Volume::NORMAL.0 * 2),
                        );
                        let mut cv = ChannelVolumes::default();
                        cv.set(2, pulse_vol);
                        context.borrow().introspect().set_sink_volume_by_name(
                            "@DEFAULT_SINK@",
                            &cv,
                            Some(Box::new(|_| {})),
                        );
                    }
                    AudioCmd::SetMute(mute) => {
                        context.borrow().introspect().set_sink_mute_by_name(
                            "@DEFAULT_SINK@",
                            mute,
                            Some(Box::new(|_| {})),
                        );
                    }
                }

                mainloop.borrow_mut().unlock();
            }
        });

        (data_rx, cmd_tx)
    }

    pub fn read_initial() -> AudioData {
        AudioData::default()
    }

    fn fetch_and_send(ctx_ref: &Rc<RefCell<Context>>, data_tx: &Sender<AudioData>) {
        let introspector = ctx_ref.borrow().introspect();
        introspector.get_sink_info_by_name("@DEFAULT_SINK@", {
            let tx = data_tx.clone();
            move |result: ListResult<&SinkInfo>| {
                if let ListResult::Item(sink) = result {
                    let vol_raw = sink.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
                    let data = AudioData {
                        volume: (vol_raw * 100.0).round() / 100.0,
                        is_muted: sink.mute,
                    };
                    let _ = tx.send_blocking(data);
                }
            }
        });
    }
}

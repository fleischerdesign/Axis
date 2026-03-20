use async_channel::{bounded, Sender};
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::{SinkInfo, SinkInputInfo};
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet, Operation};
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::mainloop::threaded::Mainloop;
use libpulse_binding::volume::{ChannelVolumes, Volume};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use super::Service;
use crate::store::ServiceStore;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SinkInputData {
    pub index: u32,
    pub name: String,
    pub icon_name: String,
    pub volume: f64,
    pub is_muted: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioData {
    pub volume: f64,
    pub is_muted: bool,
    pub sink_inputs: Vec<SinkInputData>,
}

pub enum AudioCmd {
    SetVolume(f64),
    SetMute(bool),
    SetSinkInputVolume(u32, f64),
    SetSinkInputMute(u32, bool),
}

pub struct AudioService;

impl Service for AudioService {
    type Data = AudioData;
    type Cmd = AudioCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
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

            let context = match Context::new(&*mainloop.borrow(), "axis-audio") {
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

            // Shared state: tracks the last known complete AudioData so that
            // sink-only and sink-input-only fetches can preserve the other half.
            let last_state: Rc<RefCell<AudioData>> = Rc::new(RefCell::new(AudioData::default()));

            // Subscribe für Sink + SinkInput Events
            let ctx_ref = Rc::clone(&context);
            let data_tx2 = data_tx.clone();
            let state_sub = last_state.clone();

            context.borrow_mut().set_subscribe_callback(Some(Box::new(
                move |facility: Option<Facility>, _op: Option<Operation>, _index: u32| {
                    match facility {
                        Some(Facility::Sink) => {
                            Self::fetch_sink(&ctx_ref, &data_tx2, &state_sub);
                        }
                        Some(Facility::SinkInput) => {
                            Self::fetch_sink_inputs(&ctx_ref, &data_tx2, &state_sub);
                        }
                        _ => {}
                    }
                },
            )));

            context
                .borrow_mut()
                .subscribe(InterestMaskSet::SINK | InterestMaskSet::SINK_INPUT, |_| {});

            // Ersten Zustand direkt senden (vor dem Unlock)
            Self::fetch_sink(&context, &data_tx, &last_state);
            Self::fetch_sink_inputs(&context, &data_tx, &last_state);

            mainloop.borrow_mut().unlock();

            // Command-Loop
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
                        cv.set(ChannelVolumes::CHANNELS_MAX, pulse_vol);
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
                    AudioCmd::SetSinkInputVolume(index, new_vol) => {
                        let pulse_vol = Volume(
                            ((new_vol * Volume::NORMAL.0 as f64) as u32).min(Volume::NORMAL.0 * 2),
                        );
                        let mut cv = ChannelVolumes::default();
                        cv.set(ChannelVolumes::CHANNELS_MAX, pulse_vol);
                        context.borrow().introspect().set_sink_input_volume(
                            index,
                            &cv,
                            Some(Box::new(|_| {})),
                        );
                    }
                    AudioCmd::SetSinkInputMute(index, mute) => {
                        context.borrow().introspect().set_sink_input_mute(
                            index,
                            mute,
                            Some(Box::new(|_| {})),
                        );
                    }
                }

                mainloop.borrow_mut().unlock();
            }
        });

        (ServiceStore::new(data_rx, Default::default()), cmd_tx)
    }
}

impl AudioService {

    /// Fetch default sink volume/mute. Updates shared state + sends AudioData.
    fn fetch_sink(
        ctx_ref: &Rc<RefCell<Context>>,
        data_tx: &Sender<AudioData>,
        state: &Rc<RefCell<AudioData>>,
    ) {
        let introspector = ctx_ref.borrow().introspect();
        let tx = data_tx.clone();
        let state = state.clone();

        introspector.get_sink_info_by_name("@DEFAULT_SINK@", {
            move |result: ListResult<&SinkInfo>| {
                if let ListResult::Item(sink) = result {
                    let vol_raw = sink.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
                    let vol = (vol_raw * 100.0).round() / 100.0;
                    let data = AudioData {
                        volume: vol,
                        is_muted: sink.mute,
                        sink_inputs: state.borrow().sink_inputs.clone(),
                    };
                    *state.borrow_mut() = data.clone();
                    let _ = tx.send_blocking(data);
                }
            }
        });
    }

    /// Fetch all sink-inputs. Preserves known sink volume/mute from shared state.
    fn fetch_sink_inputs(
        ctx_ref: &Rc<RefCell<Context>>,
        data_tx: &Sender<AudioData>,
        state: &Rc<RefCell<AudioData>>,
    ) {
        let introspector = ctx_ref.borrow().introspect();
        let inputs: Rc<RefCell<Vec<SinkInputData>>> = Rc::new(RefCell::new(Vec::new()));
        let tx = data_tx.clone();
        let inputs_ref = Rc::clone(&inputs);
        let state = state.clone();

        introspector.get_sink_input_info_list({
            move |result: ListResult<&SinkInputInfo>| match result {
                ListResult::Item(info) => {
                    let name = info
                        .proplist
                        .get_str("application.name")
                        .unwrap_or_else(|| "Unknown".into());
                    let icon_name = info
                        .proplist
                        .get_str("application.icon_name")
                        .unwrap_or_else(|| "audio-card-symbolic".into());
                    let vol_raw = info.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;

                    inputs_ref.borrow_mut().push(SinkInputData {
                        index: info.index,
                        name,
                        icon_name,
                        volume: (vol_raw * 100.0).round() / 100.0,
                        is_muted: info.mute,
                    });
                }
                ListResult::End => {
                    let current = state.borrow();
                    let data = AudioData {
                        sink_inputs: inputs.borrow().clone(),
                        volume: current.volume,
                        is_muted: current.is_muted,
                    };
                    drop(current);
                    *state.borrow_mut() = data.clone();
                    let _ = tx.send_blocking(data);
                }
                ListResult::Error => {}
            }
        });
    }
}

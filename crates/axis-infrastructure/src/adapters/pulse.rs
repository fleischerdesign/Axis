use axis_domain::models::audio::AudioStatus;
use axis_domain::ports::audio::{AudioProvider, AudioError, AudioStream};
use async_trait::async_trait;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::mainloop::threaded::Mainloop;
use libpulse_binding::volume::{ChannelVolumes, Volume};
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet};
use libpulse_binding::callbacks::ListResult;
use tokio::sync::{watch, mpsc};
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;
use std::time::Duration;
use log::{info, warn};

enum PulseCmd {
    SetVolume(f64),
    SetMute(bool),
    UpdateStatus,
    SetDefaultSink(u32),
    SetDefaultSource(u32),
    SetSinkInputVolume(u32, f64),
}

pub struct PulseAudioProvider {
    status_tx: watch::Sender<AudioStatus>,
    cmd_tx: mpsc::Sender<PulseCmd>,
}

impl PulseAudioProvider {
    pub async fn new() -> Result<Arc<Self>, AudioError> {
        let initial = AudioStatus::default();
        let (status_tx, status_rx) = watch::channel(initial);
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<PulseCmd>(32);

        let status_tx_c = status_tx.clone();
        let cmd_tx_c = cmd_tx.clone();

        std::thread::spawn(move || {
            let mut attempt = 0u32;
            loop {
                match Self::run_pulse_loop(&mut cmd_rx, &status_tx_c, &cmd_tx_c) {
                    Ok(()) => {
                        warn!("[audio] PulseAudio disconnected, reconnecting...");
                    }
                    Err(e) => {
                        warn!("[audio] PulseAudio error: {e}, reconnecting...");
                    }
                }
                attempt += 1;
                let delay = 2u64.pow(attempt.min(4)).min(30);
                warn!("[audio] Retry in {delay}s (attempt {attempt})");
                std::thread::sleep(Duration::from_secs(delay));
                while cmd_rx.try_recv().is_ok() {}
            }
        });

        let mut rx = status_rx;
        match tokio::time::timeout(Duration::from_secs(5), rx.changed()).await {
            Ok(Ok(())) => {
                let initial = rx.borrow();
                info!(
                    "[audio] Initialized: volume={:.0}%, muted={}",
                    initial.volume * 100.0,
                    initial.is_muted
                );
            }
            Ok(Err(_)) => {
                warn!("[audio] Watch channel closed during initialization");
            }
            Err(_) => {
                warn!("[audio] Timeout (5s) waiting for initial PulseAudio state");
            }
        }

        Ok(Arc::new(Self { status_tx, cmd_tx }))
    }

    #[allow(clippy::too_many_arguments)]
    fn run_pulse_loop(
        cmd_rx: &mut mpsc::Receiver<PulseCmd>,
        status_tx: &watch::Sender<AudioStatus>,
        cmd_tx: &mpsc::Sender<PulseCmd>,
    ) -> Result<(), String> {
        let mut mainloop = Mainloop::new().ok_or("Failed to create mainloop")?;
        let mut context = Context::new(&mainloop, "axis-shell").ok_or("Failed to create context")?;

        {
            let ml_ptr: *mut Mainloop = &mut mainloop;
            let ctx_ptr: *mut Context = &mut context;
            unsafe {
                // SAFETY: ml_ptr and ctx_ptr are raw pointers to local variables
                // (mainloop and context) that outlive this closure. The closure
                // is moved into set_state_callback and remains alive as long as
                // the context is. The mainloop is only signaled, not re-entered.
                (*ctx_ptr).set_state_callback(Some(Box::new(move || {
                    let state = (*ctx_ptr).get_state();
                    match state {
                        ContextState::Ready
                        | ContextState::Failed
                        | ContextState::Terminated => {
                            (*ml_ptr).signal(false);
                        }
                        _ => {}
                    }
                })));
            }
        }

        context.connect(None, ContextFlagSet::NOFLAGS, None)
            .map_err(|e| format!("Connect: {e:?}"))?;

        mainloop.lock();
        mainloop.start().map_err(|e| format!("Start: {e:?}"))?;

        loop {
            let state = context.get_state();
            match state {
                ContextState::Ready => {
                    info!("[audio] PulseAudio connected");
                    break;
                }
                ContextState::Failed | ContextState::Terminated => {
                    mainloop.unlock();
                    mainloop.stop();
                    return Err("Context failed during connect".into());
                }
                _ => {
                    mainloop.wait();
                }
            }
        }

        let stx_init = status_tx.clone();
        context.introspect().get_sink_info_by_name("@DEFAULT_SINK@", move |res| {
            if let ListResult::Item(sink) = res {
                let vol_raw = sink.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
                let vol = (vol_raw * 100.0).round() / 100.0;
                let status = AudioStatus {
                    volume: vol,
                    is_muted: sink.mute,
                    ..AudioStatus::default()
                };
                let _ = stx_init.send(status);
            }
        });

        let cmd_tx_loop = cmd_tx.clone();
        context.set_subscribe_callback(Some(Box::new(move |fac, _, _| {
            if let Some(Facility::Sink) = fac {
                let _ = cmd_tx_loop.try_send(PulseCmd::UpdateStatus);
            }
        })));
        context.subscribe(InterestMaskSet::SINK, |_| {});

        mainloop.unlock();

        let status_tx_c = status_tx.clone();
        while let Some(cmd) = cmd_rx.blocking_recv() {
            mainloop.lock();
            match cmd {
                PulseCmd::SetVolume(v) => {
                    info!("[audio] Set volume: {:.0}%", v * 100.0);
                    let mut cv = ChannelVolumes::default();
                    let pulse_vol = Volume(
                        ((v * Volume::NORMAL.0 as f64) as u32).min(Volume::NORMAL.0 * 2),
                    );
                    cv.set(ChannelVolumes::CHANNELS_MAX, pulse_vol);
                    context
                        .introspect()
                        .set_sink_volume_by_name("@DEFAULT_SINK@", &cv, None);
                }
                PulseCmd::SetMute(m) => {
                    info!("[audio] Set mute: {}", m);
                    context
                        .introspect()
                        .set_sink_mute_by_name("@DEFAULT_SINK@", m, None);
                }
                PulseCmd::UpdateStatus => {
                    let stx = status_tx_c.clone();
                    let prev = status_tx_c.borrow().clone();
                    context
                        .introspect()
                        .get_sink_info_by_name("@DEFAULT_SINK@", move |res| {
                            if let ListResult::Item(sink) = res {
                                let vol_raw =
                                    sink.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
                                let vol = (vol_raw * 100.0).round() / 100.0;
                                let status = AudioStatus {
                                    volume: vol,
                                    is_muted: sink.mute,
                                    sinks: prev.sinks.clone(),
                                    sources: prev.sources.clone(),
                                    sink_inputs: prev.sink_inputs.clone(),
                                };
                                let _ = stx.send(status);
                            }
                        });
                }
                PulseCmd::SetDefaultSink(id) => {
                    info!("[audio] Set default sink: {id}");
                    let name_clone;
                    {
                        let status = status_tx_c.borrow();
                        if let Some(sink) = status.sinks.iter().find(|s| s.id == id) {
                            name_clone = sink.name.clone();
                        } else {
                            mainloop.unlock();
                            continue;
                        }
                    }
                    let _ = std::process::Command::new("pactl")
                        .args(["set-default-sink", &name_clone])
                        .output();

                    let prev = status_tx_c.borrow().clone();
                    let mut new_status = prev.clone();
                    for sink in new_status.sinks.iter_mut() {
                        sink.is_default = sink.id == id;
                    }
                    let _ = status_tx_c.send(new_status);
                }
                PulseCmd::SetDefaultSource(id) => {
                    info!("[audio] Set default source: {id}");
                    let name_clone;
                    {
                        let status = status_tx_c.borrow();
                        if let Some(source) = status.sources.iter().find(|s| s.id == id) {
                            name_clone = source.name.clone();
                        } else {
                            mainloop.unlock();
                            continue;
                        }
                    }
                    let _ = std::process::Command::new("pactl")
                        .args(["set-default-source", &name_clone])
                        .output();

                    let prev = status_tx_c.borrow().clone();
                    let mut new_status = prev.clone();
                    for source in new_status.sources.iter_mut() {
                        source.is_default = source.id == id;
                    }
                    let _ = status_tx_c.send(new_status);
                }
                PulseCmd::SetSinkInputVolume(id, vol) => {
                    info!("[audio] Set sink input {id} volume: {:.0}%", vol * 100.0);
                    let mut cv = ChannelVolumes::default();
                    let pulse_vol = Volume(
                        ((vol * Volume::NORMAL.0 as f64) as u32).min(Volume::NORMAL.0 * 2),
                    );
                    cv.set(ChannelVolumes::CHANNELS_MAX, pulse_vol);
                    context.introspect().set_sink_input_volume(id, &cv, None);

                    let prev = status_tx_c.borrow().clone();
                    let mut new_status = prev.clone();
                    if let Some(input) = new_status.sink_inputs.iter_mut().find(|i| i.id == id) {
                        input.volume = vol;
                    }
                    let _ = status_tx_c.send(new_status);
                }
            }
            mainloop.unlock();
        }

        mainloop.stop();
        Ok(())
    }
}

#[async_trait]
impl AudioProvider for PulseAudioProvider {
    async fn get_status(&self) -> Result<AudioStatus, AudioError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<AudioStream, AudioError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn set_volume(&self, volume: f64) -> Result<(), AudioError> {
        let _ = self.cmd_tx.send(PulseCmd::SetVolume(volume)).await;
        Ok(())
    }

    async fn set_muted(&self, muted: bool) -> Result<(), AudioError> {
        let _ = self.cmd_tx.send(PulseCmd::SetMute(muted)).await;
        Ok(())
    }

    async fn set_default_sink(&self, id: u32) -> Result<(), AudioError> {
        let _ = self.cmd_tx.send(PulseCmd::SetDefaultSink(id)).await;
        Ok(())
    }

    async fn set_default_source(&self, id: u32) -> Result<(), AudioError> {
        let _ = self.cmd_tx.send(PulseCmd::SetDefaultSource(id)).await;
        Ok(())
    }

    async fn set_sink_input_volume(&self, id: u32, volume: f64) -> Result<(), AudioError> {
        let _ = self.cmd_tx.send(PulseCmd::SetSinkInputVolume(id, volume)).await;
        Ok(())
    }
}

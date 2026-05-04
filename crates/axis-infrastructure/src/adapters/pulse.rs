use axis_domain::models::audio::{AudioStatus, AudioDevice, SinkInput};
use axis_domain::ports::audio::{AudioProvider, AudioError, AudioStream};
use async_trait::async_trait;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::mainloop::threaded::Mainloop;
use libpulse_binding::volume::{ChannelVolumes, Volume};
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet};
use libpulse_binding::callbacks::ListResult;
use tokio::sync::{watch, mpsc};
use tokio_stream::wrappers::WatchStream;
use std::sync::{Arc, Mutex};
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

struct RefreshState {
    default_sink_name: Option<String>,
    default_source_name: Option<String>,
    sinks: Vec<(AudioDevice, f64, bool)>,
    sources: Vec<AudioDevice>,
    sink_inputs: Vec<SinkInput>,
    pending: u32,
}

impl RefreshState {
    fn new() -> Self {
        Self {
            default_sink_name: None,
            default_source_name: None,
            sinks: Vec::new(),
            sources: Vec::new(),
            sink_inputs: Vec::new(),
            pending: 4,
        }
    }

    fn mark_done(&mut self) {
        self.pending = self.pending.saturating_sub(1);
    }

    fn build_status(&mut self) -> AudioStatus {
        let default_sink = self.default_sink_name.as_deref();
        let default_source = self.default_source_name.as_deref();

        let mut sinks = Vec::with_capacity(self.sinks.len());
        let mut volume = 0.0;
        let mut is_muted = false;

        for (mut dev, vol, muted) in self.sinks.drain(..) {
            if Some(dev.name.as_str()) == default_sink {
                dev.is_default = true;
                volume = vol;
                is_muted = muted;
            }
            sinks.push(dev);
        }

        let mut sources = Vec::with_capacity(self.sources.len());
        for mut dev in self.sources.drain(..) {
            if Some(dev.name.as_str()) == default_source {
                dev.is_default = true;
            }
            sources.push(dev);
        }

        sinks.sort_by(|a, b| b.is_default.cmp(&a.is_default).then_with(|| a.name.cmp(&b.name)));
        sources.sort_by(|a, b| b.is_default.cmp(&a.is_default).then_with(|| a.name.cmp(&b.name)));

        AudioStatus {
            volume,
            is_muted,
            sinks,
            sources,
            sink_inputs: std::mem::take(&mut self.sink_inputs),
        }
    }
}

fn refresh_devices(context: &Context, status_tx: &watch::Sender<AudioStatus>) {
    let state = Arc::new(Mutex::new(RefreshState::new()));
    let stx = status_tx.clone();

    let s1 = state.clone();
    let stx1 = stx.clone();
    context.introspect().get_server_info(move |info| {
        let mut s = s1.lock().unwrap();
        s.default_sink_name = info.default_sink_name.as_ref().map(|n| n.to_string());
        s.default_source_name = info.default_source_name.as_ref().map(|n| n.to_string());
        s.mark_done();
        if s.pending == 0 {
            let status = s.build_status();
            drop(s);
            let _ = stx1.send(status);
        }
    });

    let s2 = state.clone();
    let stx2 = stx.clone();
    context.introspect().get_sink_info_list(move |res| {
        let mut s = s2.lock().unwrap();
        match res {
            ListResult::Item(sink) => {
                let name = sink.name.as_ref().map(|n| n.to_string()).unwrap_or_default();
                let desc = sink.description.as_ref().map(|d| d.to_string()).unwrap_or_default();
                let vol_raw = sink.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
                let vol = (vol_raw * 100.0).round() / 100.0;
                s.sinks.push((
                    AudioDevice {
                        id: sink.index,
                        name,
                        description: desc,
                        is_default: false,
                    },
                    vol,
                    sink.mute,
                ));
            }
            ListResult::End | ListResult::Error => {
                s.mark_done();
                if s.pending == 0 {
                    let status = s.build_status();
                    drop(s);
                    let _ = stx2.send(status);
                }
            }
        }
    });

    let s3 = state.clone();
    let stx3 = stx.clone();
    context.introspect().get_source_info_list(move |res| {
        let mut s = s3.lock().unwrap();
        match res {
            ListResult::Item(source) => {
                let name = source.name.as_ref().map(|n| n.to_string()).unwrap_or_default();
                let desc = source.description.as_ref().map(|d| d.to_string()).unwrap_or_default();
                s.sources.push(AudioDevice {
                    id: source.index,
                    name,
                    description: desc,
                    is_default: false,
                });
            }
            ListResult::End | ListResult::Error => {
                s.mark_done();
                if s.pending == 0 {
                    let status = s.build_status();
                    drop(s);
                    let _ = stx3.send(status);
                }
            }
        }
    });

    let s4 = state.clone();
    let stx4 = stx.clone();
    context.introspect().get_sink_input_info_list(move |res| {
        let mut s = s4.lock().unwrap();
        match res {
            ListResult::Item(si) => {
                let name = si.name.as_ref().map(|n| n.to_string()).unwrap_or_default();
                let vol_raw = si.volume.avg().0 as f64 / Volume::NORMAL.0 as f64;
                let vol = (vol_raw * 100.0).round() / 100.0;
                s.sink_inputs.push(SinkInput {
                    id: si.index,
                    name,
                    volume: vol,
                });
            }
            ListResult::End | ListResult::Error => {
                s.mark_done();
                if s.pending == 0 {
                    let status = s.build_status();
                    drop(s);
                    let _ = stx4.send(status);
                }
            }
        }
    });
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
        refresh_devices(&context, &stx_init);

        let cmd_tx_loop = cmd_tx.clone();
        context.set_subscribe_callback(Some(Box::new(move |fac, _, _| {
            match fac {
                Some(Facility::Sink | Facility::Source | Facility::SinkInput) => {
                    let _ = cmd_tx_loop.try_send(PulseCmd::UpdateStatus);
                }
                _ => {}
            }
        })));
        context.subscribe(InterestMaskSet::SINK | InterestMaskSet::SOURCE | InterestMaskSet::SINK_INPUT, |_| {});

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
                    refresh_devices(&context, &stx);
                }
                PulseCmd::SetDefaultSink(id) => {
                    info!("[audio] Set default sink: {id}");
                    let sink_name;
                    {
                        let status = status_tx_c.borrow();
                        if let Some(sink) = status.sinks.iter().find(|s| s.id == id) {
                            sink_name = sink.name.clone();
                        } else {
                            mainloop.unlock();
                            continue;
                        }
                    }
                    let _ = context.set_default_sink(&sink_name, |_| {});
                    let stx = status_tx_c.clone();
                    refresh_devices(&context, &stx);
                }
                PulseCmd::SetDefaultSource(id) => {
                    info!("[audio] Set default source: {id}");
                    let source_name;
                    {
                        let status = status_tx_c.borrow();
                        if let Some(source) = status.sources.iter().find(|s| s.id == id) {
                            source_name = source.name.clone();
                        } else {
                            mainloop.unlock();
                            continue;
                        }
                    }
                    let _ = context.set_default_source(&source_name, |_| {});
                    let stx = status_tx_c.clone();
                    refresh_devices(&context, &stx);
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
        self.cmd_tx.send(PulseCmd::SetVolume(volume)).await
            .map_err(|e| AudioError::ProviderError(format!("Audio channel closed: {e}")))
    }

    async fn set_muted(&self, muted: bool) -> Result<(), AudioError> {
        self.cmd_tx.send(PulseCmd::SetMute(muted)).await
            .map_err(|e| AudioError::ProviderError(format!("Audio channel closed: {e}")))
    }

    async fn set_default_sink(&self, id: u32) -> Result<(), AudioError> {
        self.cmd_tx.send(PulseCmd::SetDefaultSink(id)).await
            .map_err(|e| AudioError::ProviderError(format!("Audio channel closed: {e}")))
    }

    async fn set_default_source(&self, id: u32) -> Result<(), AudioError> {
        self.cmd_tx.send(PulseCmd::SetDefaultSource(id)).await
            .map_err(|e| AudioError::ProviderError(format!("Audio channel closed: {e}")))
    }

    async fn set_sink_input_volume(&self, id: u32, volume: f64) -> Result<(), AudioError> {
        self.cmd_tx.send(PulseCmd::SetSinkInputVolume(id, volume)).await
            .map_err(|e| AudioError::ProviderError(format!("Audio channel closed: {e}")))
    }
}

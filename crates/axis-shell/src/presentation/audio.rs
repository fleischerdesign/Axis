use std::sync::Arc;
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::audio::set_volume::SetVolumeUseCase;
use axis_application::use_cases::audio::set_default_sink::SetDefaultSinkUseCase;
use axis_application::use_cases::audio::set_default_source::SetDefaultSourceUseCase;
use axis_application::use_cases::audio::set_sink_input_volume::SetSinkInputVolumeUseCase;
use axis_domain::models::audio::AudioStatus;
use axis_domain::ports::audio::AudioProvider;
use axis_presentation::{Presenter, View};

pub(crate) fn audio_icon(status: &AudioStatus) -> &'static str {
    if status.is_muted || status.volume <= 0.01 {
        "audio-volume-muted-symbolic"
    } else if status.volume < 0.33 {
        "audio-volume-low-symbolic"
    } else if status.volume < 0.66 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    }
}

pub struct AudioPresenter {
    inner: Presenter<AudioStatus>,
    set_volume_use_case: Arc<SetVolumeUseCase>,
    set_default_sink_use_case: Arc<SetDefaultSinkUseCase>,
    set_default_source_use_case: Arc<SetDefaultSourceUseCase>,
    set_sink_input_volume_use_case: Arc<SetSinkInputVolumeUseCase>,
}

impl AudioPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn AudioProvider, AudioStatus>>,
        get_status_use_case: Arc<GetStatusUseCase<dyn AudioProvider, AudioStatus>>,
        set_volume_use_case: Arc<SetVolumeUseCase>,
        set_default_sink_use_case: Arc<SetDefaultSinkUseCase>,
        set_default_source_use_case: Arc<SetDefaultSourceUseCase>,
        set_sink_input_volume_use_case: Arc<SetSinkInputVolumeUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            match get_status_use_case.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[audio] Failed to get initial status: {e}");
                    Default::default()
                }
            }
        });

        let inner = Presenter::from_subscribe({
            let uc = subscribe_use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        }).with_initial_status(initial_status);

        Self {
            inner,
            set_volume_use_case,
            set_default_sink_use_case,
            set_default_source_use_case,
            set_sink_input_volume_use_case,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<AudioStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn handle_user_volume_change(&self, new_vol: f64) {
        if let Some(mut status) = self.inner.current() {
            if (status.volume - new_vol).abs() < 0.001 { return; }
            status.volume = new_vol;
            self.inner.update(status);
        }

        let uc = self.set_volume_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(new_vol).await {
                log::error!("[audio] set_volume failed: {e}");
            }
        });
    }

    pub fn set_default_sink(&self, id: u32) {
        let uc = self.set_default_sink_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(id).await {
                log::error!("[audio] set_default_sink failed: {e}");
            }
        });
    }

    pub fn set_default_source(&self, id: u32) {
        let uc = self.set_default_source_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(id).await {
                log::error!("[audio] set_default_source failed: {e}");
            }
        });
    }

    pub fn set_sink_input_volume(&self, id: u32, volume: f64) {
        let uc = self.set_sink_input_volume_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(id, volume).await {
                log::error!("[audio] set_sink_input_volume failed: {e}");
            }
        });
    }
}

use axis_application::use_cases::audio::set_default_sink::SetDefaultSinkUseCase;
use axis_application::use_cases::audio::set_default_source::SetDefaultSourceUseCase;
use axis_application::use_cases::audio::set_sink_input_volume::SetSinkInputVolumeUseCase;
use axis_application::use_cases::audio::set_volume::SetVolumeUseCase;
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_domain::models::audio::AudioStatus;
use axis_domain::ports::audio::AudioProvider;
use axis_presentation::{Presenter, View};
use std::sync::Arc;

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

pub struct AudioPresenterArgs {
    pub subscribe_uc: Arc<SubscribeUseCase<dyn AudioProvider, AudioStatus>>,
    pub get_status_uc: Arc<GetStatusUseCase<dyn AudioProvider, AudioStatus>>,
    pub set_volume_uc: Arc<SetVolumeUseCase>,
    pub set_default_sink_uc: Arc<SetDefaultSinkUseCase>,
    pub set_default_source_uc: Arc<SetDefaultSourceUseCase>,
    pub set_sink_input_volume_uc: Arc<SetSinkInputVolumeUseCase>,
}

impl AudioPresenter {
    pub fn new(args: AudioPresenterArgs, rt: &tokio::runtime::Runtime) -> Self {
        let AudioPresenterArgs {
            subscribe_uc,
            get_status_uc,
            set_volume_uc,
            set_default_sink_uc,
            set_default_source_uc,
            set_sink_input_volume_uc,
        } = args;

        let initial_status = rt.block_on(async {
            match get_status_uc.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[audio] Failed to get initial status: {e}");
                    Default::default()
                }
            }
        });

        let inner = Presenter::from_subscribe_use_case(subscribe_uc.clone())
            .with_initial_status(initial_status);

        Self {
            inner,
            set_volume_use_case: set_volume_uc,
            set_default_sink_use_case: set_default_sink_uc,
            set_default_source_use_case: set_default_source_uc,
            set_sink_input_volume_use_case: set_sink_input_volume_uc,
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
            if (status.volume - new_vol).abs() < 0.001 {
                return;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn audio_status(volume: f64, is_muted: bool) -> AudioStatus {
        AudioStatus {
            volume,
            is_muted,
            ..Default::default()
        }
    }

    #[test]
    fn audio_icon_muted() {
        assert!(audio_icon(&audio_status(0.5, true)).contains("muted"));
        assert!(audio_icon(&audio_status(0.0, false)).contains("muted"));
    }

    #[test]
    fn audio_icon_low() {
        assert!(audio_icon(&audio_status(0.1, false)).contains("low"));
        assert!(audio_icon(&audio_status(0.32, false)).contains("low"));
    }

    #[test]
    fn audio_icon_medium() {
        assert!(audio_icon(&audio_status(0.33, false)).contains("medium"));
        assert!(audio_icon(&audio_status(0.65, false)).contains("medium"));
    }

    #[test]
    fn audio_icon_high() {
        assert!(audio_icon(&audio_status(0.66, false)).contains("high"));
        assert!(audio_icon(&audio_status(1.0, false)).contains("high"));
    }
}

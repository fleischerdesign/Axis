use axis_application::use_cases::generic::SubscribeUseCase;
use axis_application::use_cases::mpris::next::NextTrackUseCase;
use axis_application::use_cases::mpris::play_pause::PlayPauseUseCase;
use axis_application::use_cases::mpris::previous::PreviousTrackUseCase;
use axis_domain::models::mpris::MprisStatus;
use axis_domain::ports::mpris::MprisProvider;
use axis_presentation::{Presenter, View};
use std::sync::Arc;

pub struct MprisPresenter {
    inner: Presenter<MprisStatus>,
    play_pause_uc: Arc<PlayPauseUseCase>,
    next_uc: Arc<NextTrackUseCase>,
    previous_uc: Arc<PreviousTrackUseCase>,
}

pub struct MprisPresenterArgs {
    pub subscribe_uc: Arc<SubscribeUseCase<dyn MprisProvider, MprisStatus>>,
    pub play_pause_uc: Arc<PlayPauseUseCase>,
    pub next_uc: Arc<NextTrackUseCase>,
    pub previous_uc: Arc<PreviousTrackUseCase>,
}

impl MprisPresenter {
    pub fn new(args: MprisPresenterArgs) -> Self {
        let MprisPresenterArgs {
            subscribe_uc,
            play_pause_uc,
            next_uc,
            previous_uc,
        } = args;

        let inner = Presenter::from_subscribe_use_case(subscribe_uc);

        Self {
            inner,
            play_pause_uc,
            next_uc,
            previous_uc,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<MprisStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn play_pause(&self, player_id: &str) {
        let id = player_id.to_string();
        let uc = self.play_pause_uc.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&id).await {
                log::error!("[mpris] play_pause failed: {e}");
            }
        });
    }

    pub fn next(&self, player_id: &str) {
        let id = player_id.to_string();
        let uc = self.next_uc.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&id).await {
                log::error!("[mpris] next failed: {e}");
            }
        });
    }

    pub fn previous(&self, player_id: &str) {
        let id = player_id.to_string();
        let uc = self.previous_uc.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&id).await {
                log::error!("[mpris] previous failed: {e}");
            }
        });
    }

    pub fn active_player_id(&self) -> Option<String> {
        self.inner.current().and_then(|s| s.active_player_id)
    }
}

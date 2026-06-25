use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::models::continuity::ContinuityStatus;
use axis_domain::ports::continuity::ContinuityProvider;
use axis_presentation::{Presenter, View};
use std::sync::Arc;

pub struct ContinuityPresenter {
    inner: Presenter<ContinuityStatus>,
}

impl ContinuityPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn ContinuityProvider, ContinuityStatus>>,
    ) -> Self {
        let inner = Presenter::from_subscribe_use_case(subscribe_use_case);

        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<ContinuityStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

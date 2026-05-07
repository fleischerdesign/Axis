use std::sync::Arc;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::models::clock::ClockStatus;
use axis_domain::ports::clock::ClockProvider;
use axis_presentation::{Presenter, View};

pub struct ClockPresenter {
    inner: Presenter<ClockStatus>,
}

impl ClockPresenter {
    pub fn new(use_case: Arc<SubscribeUseCase<dyn ClockProvider, ClockStatus>>) -> Self {
        let inner = Presenter::from_subscribe({
            let uc = use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        });
        Self { inner }
    }

    pub async fn bind(&self, view: Box<dyn View<ClockStatus>>) {
        self.inner.bind(view).await;
    }
}

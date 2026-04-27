use std::sync::Arc;
use axis_application::use_cases::clock::subscribe::SubscribeToClockUpdatesUseCase;
use axis_domain::models::clock::TimeStatus;
use axis_presentation::{Presenter, View};

pub struct ClockPresenter {
    inner: Presenter<TimeStatus>,
}

impl ClockPresenter {
    pub fn new(use_case: Arc<SubscribeToClockUpdatesUseCase>) -> Self {
        let inner = Presenter::from_subscribe({
            let uc = use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        });
        Self { inner }
    }

    pub async fn bind(&self, view: Box<dyn View<TimeStatus>>) {
        self.inner.bind(view).await;
    }
}

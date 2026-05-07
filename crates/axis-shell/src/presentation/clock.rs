use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::models::clock::ClockStatus;
use axis_domain::ports::clock::ClockProvider;
use axis_presentation::{Presenter, View};
use std::sync::Arc;

pub struct ClockPresenter {
    inner: Presenter<ClockStatus>,
}

impl ClockPresenter {
    pub fn new(use_case: Arc<SubscribeUseCase<dyn ClockProvider, ClockStatus>>) -> Self {
        let inner = Presenter::from_subscribe_use_case(use_case.clone());
        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<ClockStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub async fn bind(&self, view: Box<dyn View<ClockStatus>>) {
        self.add_view(view);
        self.run_sync().await;
    }
}

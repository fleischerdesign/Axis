use std::sync::Arc;

use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::models::config::AppearanceConfig;
use axis_domain::ports::appearance::AppearanceProvider;

use axis_presentation::{Presenter, View};

pub struct AppearancePresenter {
    inner: Presenter<AppearanceConfig>,
}

impl AppearancePresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn AppearanceProvider, AppearanceConfig>>,
    ) -> Self {
        let inner = Presenter::from_subscribe_use_case(subscribe_use_case);

        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<AppearanceConfig>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

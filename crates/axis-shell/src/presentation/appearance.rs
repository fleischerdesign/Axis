use std::sync::Arc;

use axis_application::use_cases::appearance::get_status::GetAppearanceStatusUseCase;
use axis_application::use_cases::appearance::subscribe::SubscribeToAppearanceUseCase;
use axis_domain::models::config::AppearanceConfig;

use axis_presentation::{Presenter, View};

pub struct AppearancePresenter {
    inner: Presenter<AppearanceConfig>,
}

impl AppearancePresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeToAppearanceUseCase>,
        get_status_use_case: Arc<GetAppearanceStatusUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt
            .block_on(async { get_status_use_case.execute().await.unwrap_or_default() });

        let inner = Presenter::from_subscribe({
            let uc = subscribe_use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        })
        .with_initial_status(initial_status);

        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<AppearanceConfig>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

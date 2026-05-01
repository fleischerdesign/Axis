use std::sync::Arc;
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_domain::models::continuity::ContinuityStatus;
use axis_domain::ports::continuity::ContinuityProvider;
use axis_presentation::{Presenter, View};

pub struct ContinuityPresenter {
    inner: Presenter<ContinuityStatus>,
}

impl ContinuityPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn ContinuityProvider, ContinuityStatus>>,
        get_status_use_case: Arc<GetStatusUseCase<dyn ContinuityProvider, ContinuityStatus>>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            match get_status_use_case.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[continuity] Failed to get initial status: {e}");
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
        })
        .with_initial_status(initial_status);

        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<ContinuityStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

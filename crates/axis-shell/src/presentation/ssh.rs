use std::sync::Arc;
use axis_domain::models::ssh::SshStatus;
use axis_domain::ports::ssh::SshProvider;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_presentation::{Presenter, View};

pub struct SshPresenter {
    inner: Presenter<SshStatus>,
}

impl SshPresenter {
    pub fn new(use_case: Arc<SubscribeUseCase<dyn SshProvider, SshStatus>>) -> Self {
        let inner = Presenter::from_subscribe({
            let uc = use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        });
        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<SshStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

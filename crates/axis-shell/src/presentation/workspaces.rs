use std::sync::Arc;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::models::workspaces::WorkspaceStatus;
use axis_domain::ports::workspaces::WorkspaceProvider;
use axis_presentation::{Presenter, View};

pub struct WorkspacePresenter {
    inner: Presenter<WorkspaceStatus>,
}

impl WorkspacePresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn WorkspaceProvider, WorkspaceStatus>>,
    ) -> Self {
        let inner = Presenter::from_subscribe({
            let uc = subscribe_use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        });

        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<WorkspaceStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn bind(&self, view: Box<dyn View<WorkspaceStatus>>) {
        self.inner.bind(view).await;
    }
}

use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::models::workspaces::WorkspaceStatus;
use axis_domain::ports::workspaces::WorkspaceProvider;
use axis_presentation::{Presenter, View};
use std::sync::Arc;

pub struct WorkspacePresenter {
    inner: Presenter<WorkspaceStatus>,
}

impl WorkspacePresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn WorkspaceProvider, WorkspaceStatus>>,
    ) -> Self {
        let inner = Presenter::from_subscribe_use_case(subscribe_use_case.clone());

        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<WorkspaceStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub async fn bind(&self, view: Box<dyn View<WorkspaceStatus>>) {
        self.inner.bind(view).await;
    }
}

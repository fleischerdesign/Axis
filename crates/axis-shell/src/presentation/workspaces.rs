use std::sync::Arc;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_application::use_cases::workspaces::focus::FocusWorkspaceUseCase;
use axis_domain::models::workspaces::WorkspaceStatus;
use axis_domain::ports::workspaces::WorkspaceProvider;
use axis_presentation::{Presenter, View};

pub trait WorkspaceView: View<WorkspaceStatus> {
    fn on_workspace_clicked(&self, f: Box<dyn Fn(u32) + Send + Sync>);
}

pub struct WorkspacePresenter {
    inner: Presenter<WorkspaceStatus>,
    focus_use_case: Arc<FocusWorkspaceUseCase>,
}

impl WorkspacePresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn WorkspaceProvider, WorkspaceStatus>>,
        focus_use_case: Arc<FocusWorkspaceUseCase>
    ) -> Self {
        let inner = Presenter::from_subscribe({
            let uc = subscribe_use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        });

        Self { inner, focus_use_case }
    }

    pub async fn bind(&self, view: Box<dyn WorkspaceView>) {
        let focus_uc = self.focus_use_case.clone();
        view.on_workspace_clicked(Box::new(move |id| {
            let uc = focus_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(id).await {
                    log::error!("[workspaces] focus failed: {e}");
                }
            });
        }));

        self.inner.bind(view).await;
    }
}

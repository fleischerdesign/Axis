use std::sync::Arc;
use axis_application::use_cases::workspaces::subscribe::SubscribeToWorkspaceUpdatesUseCase;
use axis_application::use_cases::workspaces::focus::FocusWorkspaceUseCase;
use axis_domain::models::workspaces::WorkspaceStatus;
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
        subscribe_use_case: Arc<SubscribeToWorkspaceUpdatesUseCase>,
        focus_use_case: Arc<FocusWorkspaceUseCase>
    ) -> Self {
        let uc = subscribe_use_case.clone();
        let inner = Presenter::new(move || {
            let uc = uc.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = uc.execute().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status;
                    }
                }
            })
        });

        Self { inner, focus_use_case }
    }

    pub async fn bind(&self, view: Box<dyn WorkspaceView>) {
        let focus_uc = self.focus_use_case.clone();
        view.on_workspace_clicked(Box::new(move |id| {
            let uc = focus_uc.clone();
            tokio::spawn(async move {
                let _ = uc.execute(id).await;
            });
        }));

        self.inner.bind(view).await;
    }
}

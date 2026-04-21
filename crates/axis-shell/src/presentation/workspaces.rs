use std::sync::Arc;
use futures_util::StreamExt;
use axis_application::use_cases::workspaces::subscribe::SubscribeToWorkspaceUpdatesUseCase;
use axis_application::use_cases::workspaces::focus::FocusWorkspaceUseCase;
use axis_domain::models::workspaces::Workspace;

pub trait WorkspaceView {
    fn update_workspaces(&self, workspaces: Vec<Workspace>);
    /// Ermöglicht der View, dem Presenter einen Fokus-Wunsch mitzuteilen
    fn on_workspace_clicked(&self, f: Box<dyn Fn(u32) + Send + Sync>);
}

pub struct WorkspacePresenter {
    subscribe_use_case: Arc<SubscribeToWorkspaceUpdatesUseCase>,
    focus_use_case: Arc<FocusWorkspaceUseCase>,
}

impl WorkspacePresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeToWorkspaceUpdatesUseCase>,
        focus_use_case: Arc<FocusWorkspaceUseCase>
    ) -> Self {
        Self { subscribe_use_case, focus_use_case }
    }

    pub async fn bind(&self, view: Box<dyn WorkspaceView>) {
        // 1. Auf Klicks aus der View reagieren
        let focus_uc = self.focus_use_case.clone();
        view.on_workspace_clicked(Box::new(move |id| {
            let uc = focus_uc.clone();
            tokio::spawn(async move {
                let _ = uc.execute(id).await;
            });
        }));

        // 2. Auf Updates vom System reagieren
        if let Ok(mut stream) = self.subscribe_use_case.execute().await {
            while let Some(status) = stream.next().await {
                view.update_workspaces(status.workspaces);
            }
        }
    }
}

use axis_domain::models::workspaces::{Workspace, WorkspaceStatus};
use axis_domain::ports::workspaces::{WorkspaceProvider, WorkspaceError, WorkspaceStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct MockWorkspaceProvider {
    status_tx: watch::Sender<WorkspaceStatus>,
}

impl MockWorkspaceProvider {
    pub fn new() -> Arc<Self> {
        let (tx, _) = watch::channel(WorkspaceStatus {
            workspaces: vec![
                Workspace { id: 1, name: "1".to_string(), is_active: true, is_empty: false, index: 1 },
                Workspace { id: 2, name: "2".to_string(), is_active: false, is_empty: true, index: 2 },
                Workspace { id: 3, name: "3".to_string(), is_active: false, is_empty: true, index: 3 },
                Workspace { id: 4, name: "4".to_string(), is_active: false, is_empty: true, index: 4 },
                Workspace { id: 5, name: "5".to_string(), is_active: false, is_empty: true, index: 5 },
            ],
            overview_open: false,
        });

        Arc::new(Self { status_tx: tx })
    }

    pub fn simulate_active(&self, active_id: u32) {
        let mut status = self.status_tx.borrow().clone();
        for ws in status.workspaces.iter_mut() {
            ws.is_active = ws.id == active_id;
        }
        let _ = self.status_tx.send(status);
    }
}

#[async_trait]
impl WorkspaceProvider for MockWorkspaceProvider {
    async fn get_status(&self) -> Result<WorkspaceStatus, WorkspaceError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<WorkspaceStream, WorkspaceError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn focus_workspace(&self, id: u32) -> Result<(), WorkspaceError> {
        self.simulate_active(id);
        Ok(())
    }

    async fn toggle_overview(&self) -> Result<(), WorkspaceError> {
        Ok(())
    }
}

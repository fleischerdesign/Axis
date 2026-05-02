use axis_domain::models::workspaces::{Workspace, WorkspaceStatus};
use axis_domain::ports::workspaces::{WorkspaceProvider, WorkspaceError, WorkspaceStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;
use niri_ipc::{socket::Socket, Event, Request, Response, Action, WorkspaceReferenceArg};

pub struct NiriWorkspaceProvider {
    status_tx: watch::Sender<WorkspaceStatus>,
}

impl NiriWorkspaceProvider {
    pub async fn new() -> Result<Arc<Self>, WorkspaceError> {
        let (initial_status, _query_sock) = {
            let mut sock = Socket::connect()
                .map_err(|e| WorkspaceError::ProviderError(e.to_string()))?;
            
            let response = sock.send(Request::Workspaces)
                .map_err(|e| WorkspaceError::ProviderError(e.to_string()))?
                .map_err(|e| WorkspaceError::ProviderError(e))?;
            
            let status = Self::map_workspaces_response(response)?;
            (status, sock)
        };

        let (tx, _) = watch::channel(initial_status);
        let provider = Arc::new(Self { status_tx: tx });

        let provider_clone = provider.clone();
        std::thread::spawn(move || {
            loop {
                if let Ok(mut events_sock) = Socket::connect() {
                    if let Ok(Ok(Response::Handled)) = events_sock.send(Request::EventStream) {
                        let mut read_event = events_sock.read_events();
                        while let Ok(event) = read_event() {
                            match event {
                                Event::WorkspacesChanged { workspaces } => {
                                    let overview_open = provider_clone.status_tx.borrow().overview_open;
                                    let status = WorkspaceStatus {
                                        workspaces: workspaces.into_iter().map(Self::map_workspace).collect(),
                                        overview_open,
                                    };
                                    let _ = provider_clone.status_tx.send(status);
                                }
                                Event::WorkspaceActivated { id, .. } => {
                                    let mut status = provider_clone.status_tx.borrow().clone();
                                    for ws in &mut status.workspaces {
                                        ws.is_active = ws.id == id as u32;
                                    }
                                    let _ = provider_clone.status_tx.send(status);
                                }
                                Event::OverviewOpenedOrClosed { is_open } => {
                                    let mut status = provider_clone.status_tx.borrow().clone();
                                    status.overview_open = is_open;
                                    let _ = provider_clone.status_tx.send(status);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        Ok(provider)
    }

    fn map_workspaces_response(response: Response) -> Result<WorkspaceStatus, WorkspaceError> {
        match response {
            Response::Workspaces(ws_list) => {
                let workspaces = ws_list.into_iter().map(Self::map_workspace).collect();
                Ok(WorkspaceStatus { workspaces, overview_open: false })
            }
            _ => Err(WorkspaceError::ProviderError("Unexpected response from Niri".to_string())),
        }
    }

    fn map_workspace(ws: niri_ipc::Workspace) -> Workspace {
        Workspace {
            id: ws.id as u32,
            name: ws.name.unwrap_or_else(|| ws.id.to_string()),
            is_active: ws.is_active,
            is_empty: false,
            index: ws.id as u32,
        }
    }
}

#[async_trait]
impl WorkspaceProvider for NiriWorkspaceProvider {
    async fn get_status(&self) -> Result<WorkspaceStatus, WorkspaceError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<WorkspaceStream, WorkspaceError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn focus_workspace(&self, id: u32) -> Result<(), WorkspaceError> {
        let mut sock = Socket::connect()
            .map_err(|e| WorkspaceError::ProviderError(e.to_string()))?;
        sock.send(Request::Action(Action::FocusWorkspace {
            reference: WorkspaceReferenceArg::Id(id as u64),
        }))
        .map_err(|e| WorkspaceError::ProviderError(e.to_string()))?
        .map_err(|e| WorkspaceError::ProviderError(e))?;
        Ok(())
    }

    async fn toggle_overview(&self) -> Result<(), WorkspaceError> {
        let mut sock = Socket::connect()
            .map_err(|e| WorkspaceError::ProviderError(e.to_string()))?;
        sock.send(Request::Action(Action::ToggleOverview {}))
            .map_err(|e| WorkspaceError::ProviderError(e.to_string()))?
            .map_err(|e| WorkspaceError::ProviderError(e))?;
        Ok(())
    }
}

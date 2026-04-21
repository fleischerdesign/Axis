use axis_domain::models::dnd::DndStatus;
use axis_domain::ports::dnd::{DndProvider, DndError, DndStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct MockDndProvider {
    status_tx: watch::Sender<DndStatus>,
}

impl MockDndProvider {
    pub fn new() -> Arc<Self> {
        let (status_tx, _) = watch::channel(DndStatus { enabled: false });
        Arc::new(Self { status_tx })
    }
}

#[async_trait]
impl DndProvider for MockDndProvider {
    async fn get_status(&self) -> Result<DndStatus, DndError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<DndStream, DndError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), DndError> {
        self.status_tx.send_modify(|s| s.enabled = enabled);
        Ok(())
    }
}

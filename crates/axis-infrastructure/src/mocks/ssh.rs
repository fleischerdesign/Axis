use axis_domain::models::ssh::SshStatus;
use axis_domain::ports::ssh::{SshProvider, SshError, SshStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct MockSshProvider {
    status_tx: watch::Sender<SshStatus>,
}

impl MockSshProvider {
    pub fn new() -> Arc<Self> {
        let (status_tx, _) = watch::channel(SshStatus::default());
        Arc::new(Self { status_tx })
    }
}

#[async_trait]
impl SshProvider for MockSshProvider {
    async fn get_status(&self) -> Result<SshStatus, SshError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<SshStream, SshError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }
}

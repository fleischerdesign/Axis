use axis_domain::models::lock::LockStatus;
use axis_domain::ports::lock::{LockProvider, LockError, LockStream};
use async_trait::async_trait;
use log::info;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct MockLockProvider {
    status_tx: watch::Sender<LockStatus>,
}

impl MockLockProvider {
    pub fn new() -> Arc<Self> {
        let initial = LockStatus {
            is_locked: false,
            is_supported: true,
        };
        let (tx, _) = watch::channel(initial);
        Arc::new(Self { status_tx: tx })
    }
}

#[async_trait]
impl LockProvider for MockLockProvider {
    async fn is_supported(&self) -> Result<bool, LockError> {
        Ok(true)
    }

    async fn lock(&self) -> Result<(), LockError> {
        let _ = self.status_tx.send(LockStatus {
            is_locked: true,
            is_supported: true,
        });
        info!("[mock-lock] lock_session");
        Ok(())
    }

    async fn unlock(&self) -> Result<(), LockError> {
        let _ = self.status_tx.send(LockStatus {
            is_locked: false,
            is_supported: true,
        });
        info!("[mock-lock] unlock_session");
        Ok(())
    }

    async fn authenticate(&self, password: &str) -> Result<bool, LockError> {
        Ok(password == "password")
    }

    async fn subscribe(&self) -> Result<LockStream, LockError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }
}

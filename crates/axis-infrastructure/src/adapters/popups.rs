use axis_domain::models::popups::{PopupType, PopupStatus};
use axis_domain::ports::popups::{PopupProvider, PopupError, PopupStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct LocalPopupProvider {
    status_tx: watch::Sender<PopupStatus>,
}

impl LocalPopupProvider {
    pub fn new() -> Arc<Self> {
        let (tx, _) = watch::channel(PopupStatus { active_popup: None });
        Arc::new(Self { status_tx: tx })
    }
}

#[async_trait]
impl PopupProvider for LocalPopupProvider {
    async fn get_status(&self) -> Result<PopupStatus, PopupError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<PopupStream, PopupError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn open_popup(&self, popup_type: PopupType) -> Result<(), PopupError> {
        let _ = self.status_tx.send(PopupStatus { active_popup: Some(popup_type) });
        Ok(())
    }

    async fn close_popup(&self) -> Result<(), PopupError> {
        let _ = self.status_tx.send(PopupStatus { active_popup: None });
        Ok(())
    }

    async fn toggle_popup(&self, popup_type: PopupType) -> Result<(), PopupError> {
        let current = self.status_tx.borrow().active_popup;
        if current == Some(popup_type) {
            self.close_popup().await
        } else {
            self.open_popup(popup_type).await
        }
    }
}

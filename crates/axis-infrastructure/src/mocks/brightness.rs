use axis_domain::models::brightness::BrightnessStatus;
use axis_domain::ports::brightness::{BrightnessProvider, BrightnessError, BrightnessStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct MockBrightnessProvider {
    status_tx: watch::Sender<BrightnessStatus>,
}

impl MockBrightnessProvider {
    pub fn new() -> Arc<Self> {
        let (tx, _) = watch::channel(BrightnessStatus {
            percentage: 60.0,
            has_backlight: true,
        });
        Arc::new(Self { status_tx: tx })
    }
}

#[async_trait]
impl BrightnessProvider for MockBrightnessProvider {
    async fn get_status(&self) -> Result<BrightnessStatus, BrightnessError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<BrightnessStream, BrightnessError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_brightness(&self, percentage: f64) -> Result<(), BrightnessError> {
        let _ = self.status_tx.send(BrightnessStatus {
            percentage,
            has_backlight: true,
        });
        Ok(())
    }
}

use axis_domain::models::airplane::AirplaneStatus;
use axis_domain::ports::airplane::{AirplaneError, AirplaneProvider, AirplaneStream};
use async_trait::async_trait;
use std::sync::Mutex;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct MockAirplaneProvider {
    status: Mutex<AirplaneStatus>,
    status_tx: watch::Sender<AirplaneStatus>,
}

impl MockAirplaneProvider {
    pub fn new() -> Self {
        let status = AirplaneStatus {
            enabled: false,
            available: true,
        };
        let (status_tx, _) = watch::channel(status.clone());
        Self {
            status: Mutex::new(status),
            status_tx,
        }
    }
}

#[async_trait]
impl AirplaneProvider for MockAirplaneProvider {
    async fn get_status(&self) -> Result<AirplaneStatus, AirplaneError> {
        Ok(self.status.lock().unwrap().clone())
    }

    async fn subscribe(&self) -> Result<AirplaneStream, AirplaneError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), AirplaneError> {
        let mut status = self.status.lock().unwrap();
        status.enabled = enabled;
        let clone = status.clone();
        drop(status);
        self.status_tx.send_modify(|s| *s = clone);
        Ok(())
    }
}

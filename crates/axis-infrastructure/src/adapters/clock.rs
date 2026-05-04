use axis_domain::models::clock::TimeStatus;
use axis_domain::ports::clock::{ClockProvider, ClockError, ClockStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use chrono::Local;
use std::sync::Arc;
use std::time::Duration;

pub struct SystemClockProvider {
    status_tx: watch::Sender<TimeStatus>,
}

impl SystemClockProvider {
    pub fn new() -> Arc<Self> {
        let (tx, _) = watch::channel(TimeStatus {
            current_time: Local::now(),
        });
        
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let _ = tx_clone.send(TimeStatus {
                    current_time: Local::now(),
                });
            }
        });

        Arc::new(Self { status_tx: tx })
    }
}

#[async_trait]
impl ClockProvider for SystemClockProvider {
    async fn get_status(&self) -> Result<TimeStatus, ClockError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<ClockStream, ClockError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }
}

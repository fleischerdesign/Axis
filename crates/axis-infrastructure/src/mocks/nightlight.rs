use axis_domain::models::nightlight::NightlightStatus;
use axis_domain::ports::nightlight::{NightlightProvider, NightlightError, NightlightStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct MockNightlightProvider {
    status_tx: watch::Sender<NightlightStatus>,
}

impl MockNightlightProvider {
    pub fn new() -> Arc<Self> {
        let (status_tx, _) = watch::channel(NightlightStatus {
            enabled: false,
            available: true,
            temp_day: 6500,
            temp_night: 4500,
            schedule_enabled: false,
            sunrise: String::new(),
            sunset: String::new(),
        });
        Arc::new(Self { status_tx })
    }
}

#[async_trait]
impl NightlightProvider for MockNightlightProvider {
    async fn get_status(&self) -> Result<NightlightStatus, NightlightError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<NightlightStream, NightlightError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), NightlightError> {
        self.status_tx.send_modify(|s| s.enabled = enabled);
        Ok(())
    }

    async fn set_temp_day(&self, temp: u32) -> Result<(), NightlightError> {
        self.status_tx.send_modify(|s| s.temp_day = temp);
        Ok(())
    }

    async fn set_temp_night(&self, temp: u32) -> Result<(), NightlightError> {
        self.status_tx.send_modify(|s| s.temp_night = temp);
        Ok(())
    }

    async fn set_schedule(&self, sunrise: &str, sunset: &str) -> Result<(), NightlightError> {
        let sunrise = sunrise.to_string();
        let sunset = sunset.to_string();
        self.status_tx.send_modify(move |s| {
            s.schedule_enabled = true;
            s.sunrise = sunrise;
            s.sunset = sunset;
        });
        Ok(())
    }
}

use async_trait::async_trait;
use axis_domain::models::nightlight::NightlightStatus;
use axis_domain::ports::nightlight::{NightlightError, NightlightProvider, NightlightStream};
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

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
            sunrise: chrono::NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            sunset: chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
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
        let sunrise = chrono::NaiveTime::parse_from_str(sunrise, "%H:%M")
            .unwrap_or_else(|_| chrono::NaiveTime::from_hms_opt(6, 0, 0).unwrap());
        let sunset = chrono::NaiveTime::parse_from_str(sunset, "%H:%M")
            .unwrap_or_else(|_| chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap());
        self.status_tx.send_modify(move |s| {
            s.schedule_enabled = true;
            s.sunrise = sunrise;
            s.sunset = sunset;
        });
        Ok(())
    }
}

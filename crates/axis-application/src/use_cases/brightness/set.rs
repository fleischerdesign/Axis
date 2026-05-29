use axis_domain::ports::brightness::{BrightnessError, BrightnessProvider};
use log::debug;
use std::sync::Arc;

pub struct SetBrightnessUseCase {
    provider: Arc<dyn BrightnessProvider>,
}

impl SetBrightnessUseCase {
    pub fn new(provider: Arc<dyn BrightnessProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, percentage: f64) -> Result<(), BrightnessError> {
        let percentage = percentage.clamp(0.0, 1.0);
        debug!("[use-case] Setting screen brightness to {:.2}", percentage);
        self.provider.set_brightness(percentage).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axis_infrastructure::mocks::brightness::MockBrightnessProvider;

    #[tokio::test]
    async fn set_brightness_normal() {
        let mock = MockBrightnessProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetBrightnessUseCase::new(mock.clone());
        uc.execute(0.5).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!((status.percentage - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn set_brightness_clamps_to_max() {
        let mock = MockBrightnessProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetBrightnessUseCase::new(mock.clone());
        uc.execute(1.5).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!((status.percentage - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn set_brightness_clamps_to_min() {
        let mock = MockBrightnessProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetBrightnessUseCase::new(mock.clone());
        uc.execute(-0.5).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!((status.percentage - 0.0).abs() < 0.01);
    }
}

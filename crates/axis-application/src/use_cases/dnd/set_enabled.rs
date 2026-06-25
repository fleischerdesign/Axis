use axis_domain::ports::dnd::{DndError, DndProvider};
use log::debug;
use std::sync::Arc;

pub struct SetDndEnabledUseCase {
    provider: Arc<dyn DndProvider>,
}

impl SetDndEnabledUseCase {
    pub fn new(provider: Arc<dyn DndProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), DndError> {
        debug!(
            "[use-case] Setting Do-Not-Disturb (DND) mode to: {}",
            enabled
        );
        self.provider.set_enabled(enabled).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axis_infrastructure::mocks::dnd::MockDndProvider;

    #[tokio::test]
    async fn set_dnd_enabled() {
        let mock = MockDndProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetDndEnabledUseCase::new(mock.clone());
        uc.execute(true).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!(status.enabled);
    }

    #[tokio::test]
    async fn set_dnd_disabled() {
        let mock = MockDndProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetDndEnabledUseCase::new(mock.clone());
        uc.execute(false).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!(!status.enabled);
    }
}

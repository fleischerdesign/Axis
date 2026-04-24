use axis_domain::ports::popups::{PopupProvider, PopupError, PopupStream};
use axis_domain::models::popups::PopupType;
use std::sync::Arc;
use log::debug;

pub struct SubscribeToPopupUpdatesUseCase {
    provider: Arc<dyn PopupProvider>,
}

impl SubscribeToPopupUpdatesUseCase {
    pub fn new(provider: Arc<dyn PopupProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<PopupStream, PopupError> {
        self.provider.subscribe().await
    }
}

pub struct TogglePopupUseCase {
    provider: Arc<dyn PopupProvider>,
}

impl TogglePopupUseCase {
    pub fn new(provider: Arc<dyn PopupProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, popup_type: PopupType) -> Result<(), PopupError> {
        debug!("[use-case] Toggling popup: {:?}", popup_type);
        self.provider.toggle_popup(popup_type).await
    }
}

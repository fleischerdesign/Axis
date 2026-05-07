use axis_domain::models::popups::PopupType;
use axis_domain::ports::popups::PopupProvider;
use log::debug;
use std::sync::Arc;

pub struct TogglePopupUseCase {
    provider: Arc<dyn PopupProvider>,
}

impl TogglePopupUseCase {
    pub fn new(provider: Arc<dyn PopupProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(
        &self,
        popup_type: PopupType,
    ) -> Result<(), axis_domain::ports::popups::PopupError> {
        debug!("[use-case] Toggling popup: {:?}", popup_type);
        self.provider.toggle_popup(popup_type).await
    }
}

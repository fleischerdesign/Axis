use std::sync::Arc;
use axis_application::use_cases::popups::SubscribeToPopupUpdatesUseCase;
use axis_domain::models::popups::{PopupType, PopupStatus};
use axis_presentation::{Presenter, View};
use crate::widgets::popup_base::PopupContainer;

pub trait PopupView: View<PopupStatus> {
    fn get_type(&self) -> PopupType;
    fn popup_container(&self) -> PopupContainer;
    fn popup_window(&self) -> gtk4::ApplicationWindow;

    fn show(&self) {
        self.popup_container().animate_show(&self.popup_window());
    }

    fn hide(&self) {
        self.popup_container().animate_hide(&self.popup_window());
    }

    fn handle_status(&self, status: &PopupStatus) {
        if status.active_popup == Some(self.get_type()) {
            self.show();
        } else {
            self.hide();
        }
    }
}

pub struct PopupPresenter {
    inner: Presenter<PopupStatus>,
}

impl PopupPresenter {
    pub fn new(subscribe_use_case: Arc<SubscribeToPopupUpdatesUseCase>) -> Self {
        let uc = subscribe_use_case.clone();
        let inner = Presenter::new(move || {
            let uc = uc.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = uc.execute().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status;
                    }
                }
            })
        });

        Self { inner }
    }

    pub fn add_popup(&self, popup: Box<dyn PopupView>) {
        self.inner.add_view(popup);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

use std::sync::Arc;
use futures_util::StreamExt;
use axis_application::use_cases::popups::SubscribeToPopupUpdatesUseCase;
use axis_domain::models::popups::PopupType;
use std::cell::RefCell;
use crate::widgets::popup_base::PopupContainer;

pub trait PopupView {
    fn get_type(&self) -> PopupType;
    fn popup_container(&self) -> PopupContainer;
    fn popup_window(&self) -> gtk4::ApplicationWindow;

    fn show(&self) {
        self.popup_container().animate_show(&self.popup_window());
    }

    fn hide(&self) {
        self.popup_container().animate_hide(&self.popup_window());
    }
}

pub struct PopupPresenter {
    subscribe_use_case: Arc<SubscribeToPopupUpdatesUseCase>,
    last_active: RefCell<Option<PopupType>>,
}

impl PopupPresenter {
    pub fn new(subscribe_use_case: Arc<SubscribeToPopupUpdatesUseCase>) -> Self {
        Self { 
            subscribe_use_case,
            last_active: RefCell::new(None),
        }
    }

    pub async fn bind(&self, popups: Vec<Box<dyn PopupView>>) {
        if let Ok(mut stream) = self.subscribe_use_case.execute().await {
            while let Some(status) = stream.next().await {
                let current = status.active_popup;
                let last = *self.last_active.borrow();

                if current == last {
                    continue;
                }

                for popup in &popups {
                    let p_type = popup.get_type();
                    if Some(p_type) == current {
                        popup.show();
                    } else if Some(p_type) == last {
                        popup.hide();
                    }
                }

                *self.last_active.borrow_mut() = current;
            }
        }
    }
}

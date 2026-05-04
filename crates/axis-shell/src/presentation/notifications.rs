use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::notifications::close_notification::CloseNotificationUseCase;
use axis_application::use_cases::notifications::invoke_action::InvokeNotificationActionUseCase;
use axis_domain::models::notifications::NotificationStatus;
use axis_domain::ports::notifications::NotificationProvider;
use axis_presentation::{Presenter, View};

pub struct NotificationPresenter {
    inner: Presenter<NotificationStatus>,
    close_use_case: Arc<CloseNotificationUseCase>,
    invoke_action_use_case: Arc<InvokeNotificationActionUseCase>,
    toast_view: RefCell<Option<Rc<dyn NotificationPopupAware>>>,
    archive_view: RefCell<Option<Rc<dyn NotificationPopupAware>>>,
}

pub trait NotificationPopupAware {
    fn set_popup_open(&self, open: bool);
}

impl NotificationPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn NotificationProvider, NotificationStatus>>,
        get_status_use_case: Arc<GetStatusUseCase<dyn NotificationProvider, NotificationStatus>>,
        close_use_case: Arc<CloseNotificationUseCase>,
        invoke_action_use_case: Arc<InvokeNotificationActionUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            match get_status_use_case.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[notifications] Failed to get initial status: {e}");
                    Default::default()
                }
            }
        });

        let inner = Presenter::from_subscribe({
            let uc = subscribe_use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        }).with_initial_status(initial_status);

        Self {
            inner,
            close_use_case,
            invoke_action_use_case,
            toast_view: RefCell::new(None),
            archive_view: RefCell::new(None),
        }
    }

    pub fn add_view(&self, view: Box<dyn View<NotificationStatus>>) {
        self.inner.add_view(view);
    }

    pub fn register_toast(&self, toast: Rc<dyn NotificationPopupAware>) {
        *self.toast_view.borrow_mut() = Some(toast);
    }

    pub fn register_archive(&self, archive: Rc<dyn NotificationPopupAware>) {
        *self.archive_view.borrow_mut() = Some(archive);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn close_notification(&self, id: u32) {
        let uc = self.close_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(id).await {
                log::error!("[notifications] close_notification failed: {e}");
            }
        });
    }

    pub fn invoke_action(&self, id: u32, action_key: String, user_input: Option<String>) {
        let uc = self.invoke_action_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(id, &action_key, user_input).await {
                log::error!("[notifications] invoke_action failed: {e}");
            }
        });
    }

    pub fn set_popup_open(&self, open: bool) {
        if let Some(toast) = self.toast_view.borrow().as_ref() {
            toast.set_popup_open(open);
        }
        if let Some(archive) = self.archive_view.borrow().as_ref() {
            archive.set_popup_open(open);
        }
    }
}

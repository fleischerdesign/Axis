use axis_application::use_cases::generic::SubscribeUseCase;
use axis_application::use_cases::notifications::close_notification::CloseNotificationUseCase;
use axis_application::use_cases::notifications::invoke_action::InvokeNotificationActionUseCase;
use axis_domain::models::notifications::NotificationStatus;
use axis_domain::ports::notifications::NotificationProvider;
use axis_presentation::{Presenter, View};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

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

pub struct NotificationPresenterArgs {
    pub subscribe_uc: Arc<SubscribeUseCase<dyn NotificationProvider, NotificationStatus>>,
    pub close_uc: Arc<CloseNotificationUseCase>,
    pub invoke_action_uc: Arc<InvokeNotificationActionUseCase>,
}

impl NotificationPresenter {
    pub fn new(args: NotificationPresenterArgs) -> Self {
        let NotificationPresenterArgs {
            subscribe_uc,
            close_uc,
            invoke_action_uc,
        } = args;

        let inner = Presenter::from_subscribe_use_case(subscribe_uc);

        Self {
            inner,
            close_use_case: close_uc,
            invoke_action_use_case: invoke_action_uc,
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

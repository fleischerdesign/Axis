use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;
use axis_domain::models::notifications::NotificationStatus;
use axis_domain::ports::notifications::NotificationService;
use super::presenter::{Presenter, View};

pub struct NotificationPresenter {
    inner: Presenter<dyn View<NotificationStatus>, NotificationStatus>,
    service: Arc<dyn NotificationService>,
    toast_view: RefCell<Option<Rc<dyn NotificationPopupAware>>>,
    archive_view: RefCell<Option<Rc<dyn NotificationPopupAware>>>,
}

pub trait NotificationPopupAware {
    fn set_popup_open(&self, open: bool);
}

impl NotificationPresenter {
    pub fn new(service: Arc<dyn NotificationService>) -> Self {
        let svc = service.clone();
        let inner = Presenter::new(move || {
            let svc = svc.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = svc.subscribe().await {
                    while let Some(item) = futures_util::StreamExt::next(&mut stream).await {
                        yield item;
                    }
                }
            })
        });

        Self {
            inner,
            service,
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
        self.inner.run().await;
    }

    pub fn close_notification(&self, id: u32) {
        let svc = self.service.clone();
        tokio::spawn(async move {
            let _ = svc.close_notification(id).await;
        });
    }

    pub fn invoke_action(&self, id: u32, action_key: String) {
        let svc = self.service.clone();
        tokio::spawn(async move {
            let _ = svc.invoke_action(id, &action_key).await;
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

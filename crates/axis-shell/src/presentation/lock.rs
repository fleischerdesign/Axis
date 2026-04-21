use std::rc::Rc;
use std::sync::Arc;

use gtk4::glib;
use crate::presentation::presenter::{Presenter, View};
use axis_application::use_cases::lock::lock::LockSessionUseCase;
use axis_application::use_cases::lock::unlock::UnlockSessionUseCase;
use axis_application::use_cases::lock::authenticate::AuthenticateUseCase;
use axis_application::use_cases::lock::subscribe::SubscribeToLockUpdatesUseCase;
use axis_domain::models::lock::LockStatus;

pub trait LockView: View<LockStatus> {
    fn on_auth_result(&self, success: bool);
}

impl<T: LockView + ?Sized> LockView for Rc<T> {
    fn on_auth_result(&self, success: bool) {
        (**self).on_auth_result(success);
    }
}

impl<T: LockView + ?Sized> LockView for Arc<T> {
    fn on_auth_result(&self, success: bool) {
        (**self).on_auth_result(success);
    }
}

pub struct LockPresenter {
    inner: Presenter<dyn LockView, LockStatus>,
    lock_uc: Arc<LockSessionUseCase>,
    unlock_uc: Arc<UnlockSessionUseCase>,
    authenticate_uc: Arc<AuthenticateUseCase>,
}

impl LockPresenter {
    pub fn new(
        subscribe_uc: Arc<SubscribeToLockUpdatesUseCase>,
        lock_uc: Arc<LockSessionUseCase>,
        unlock_uc: Arc<UnlockSessionUseCase>,
        authenticate_uc: Arc<AuthenticateUseCase>,
    ) -> Self {
        let inner = Presenter::new(move || {
            let uc = subscribe_uc.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = uc.execute().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status;
                    }
                }
            })
        });

        Self {
            inner,
            lock_uc,
            unlock_uc,
            authenticate_uc,
        }
    }

    pub fn add_view(&self, view: Box<dyn LockView>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run().await;
    }

    pub fn lock(&self) {
        let uc = self.lock_uc.clone();
        tokio::spawn(async move {
            let _ = uc.execute().await;
        });
    }

    pub fn unlock(&self) {
        let uc = self.unlock_uc.clone();
        tokio::spawn(async move {
            let _ = uc.execute().await;
        });
    }

    pub fn authenticate(&self, password: &str, on_result: Rc<dyn Fn(bool)>) {
        let uc = self.authenticate_uc.clone();
        let password = password.to_string();
        glib::spawn_future_local(async move {
            let success = uc.execute(&password).await.unwrap_or(false);
            on_result(success);
        });
    }
}

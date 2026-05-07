use std::rc::Rc;
use std::sync::Arc;

use gtk4::glib;
use axis_presentation::{Presenter, View};
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_application::use_cases::lock::session::LockSessionUseCase;
use axis_application::use_cases::lock::unlock::UnlockSessionUseCase;
use axis_application::use_cases::lock::authenticate::AuthenticateUseCase;
use axis_domain::models::lock::LockStatus;
use axis_domain::ports::lock::LockProvider;

pub struct LockPresenter {
    inner: Presenter<LockStatus>,
    lock_uc: Arc<LockSessionUseCase>,
    unlock_uc: Arc<UnlockSessionUseCase>,
    authenticate_uc: Arc<AuthenticateUseCase>,
}

impl LockPresenter {
    pub fn new(
        subscribe_uc: Arc<SubscribeUseCase<dyn LockProvider, LockStatus>>,
        lock_uc: Arc<LockSessionUseCase>,
        unlock_uc: Arc<UnlockSessionUseCase>,
        authenticate_uc: Arc<AuthenticateUseCase>,
    ) -> Self {
        let inner = Presenter::from_subscribe_use_case(subscribe_uc.clone());

        Self {
            inner,
            lock_uc,
            unlock_uc,
            authenticate_uc,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<LockStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn lock(&self) {
        let uc = self.lock_uc.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute().await {
                log::error!("[lock] lock failed: {e}");
            }
        });
    }

    pub fn unlock(&self) {
        let uc = self.unlock_uc.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute().await {
                log::error!("[lock] unlock failed: {e}");
            }
        });
    }

    pub fn authenticate(&self, password: &str, on_result: Rc<dyn Fn(bool)>) {
        let uc = self.authenticate_uc.clone();
        let password = password.to_string();
        // glib::spawn_future_local is required here because the async closure captures
        // Rc-based state (Presenter<S>) which is !Send. This is GTK's single-threaded
        // UI model and is architecturally intentional.
        glib::spawn_future_local(async move {
            let success = match uc.execute(&password).await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[lock] authenticate failed: {e}");
                    false
                }
            };
            on_result(success);
        });
    }
}

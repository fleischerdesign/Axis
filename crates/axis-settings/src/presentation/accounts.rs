use axis_domain::models::cloud::CloudStatus;
use axis_presentation::{Presenter, View};
use axis_application::use_cases::cloud::subscribe::SubscribeToCloudUpdatesUseCase;
use axis_application::use_cases::cloud::authenticate::AuthenticateAccountUseCase;
use std::rc::Rc;
use std::sync::Arc;

pub struct AccountsPresenter {
    presenter: Presenter<CloudStatus>,
    authenticate_uc: Arc<AuthenticateAccountUseCase>,
}

pub trait AccountsView: View<CloudStatus> {
    fn on_auth_error(&self, error: String);
}

impl<T: AccountsView + ?Sized> AccountsView for Rc<T> {
    fn on_auth_error(&self, error: String) {
        (**self).on_auth_error(error);
    }
}

impl AccountsPresenter {
    pub fn new(
        subscribe_uc: Arc<SubscribeToCloudUpdatesUseCase>,
        authenticate_uc: Arc<AuthenticateAccountUseCase>,
    ) -> Self {
        let sub = subscribe_uc.clone();
        let presenter = Presenter::new(move || {
            let sub = sub.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = sub.execute().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status;
                    }
                }
            })
        });

        Self {
            presenter,
            authenticate_uc,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<CloudStatus>>) {
        self.presenter.add_view(view);
    }

    pub async fn run(&self) {
        self.presenter.run_sync().await;
    }

    pub fn add_google_account(&self) {
        let uc = self.authenticate_uc.clone();
        let scopes = vec![
            "https://www.googleapis.com/auth/userinfo.profile".to_string(),
            "https://www.googleapis.com/auth/userinfo.email".to_string(),
            "https://www.googleapis.com/auth/calendar.readonly".to_string(),
            "https://www.googleapis.com/auth/tasks".to_string(),
        ];

        tokio::spawn(async move {
            if let Err(e) = uc.execute(scopes).await {
                log::error!("[accounts] Auth failed: {e}");
            }
        });
    }
}

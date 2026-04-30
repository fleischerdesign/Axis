use std::sync::Arc;
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::network::connect_to_ap::ConnectToApUseCase;
use axis_application::use_cases::network::disconnect_wifi::DisconnectWifiUseCase;
use axis_domain::models::network::NetworkStatus;
use axis_domain::ports::network::NetworkProvider;
use axis_presentation::{Presenter, View};

pub struct NetworkPresenter {
    inner: Presenter<NetworkStatus>,
    connect_use_case: Arc<ConnectToApUseCase>,
    disconnect_use_case: Arc<DisconnectWifiUseCase>,
}

impl NetworkPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn NetworkProvider, NetworkStatus>>,
        get_status_use_case: Arc<GetStatusUseCase<dyn NetworkProvider, NetworkStatus>>,
        connect_use_case: Arc<ConnectToApUseCase>,
        disconnect_use_case: Arc<DisconnectWifiUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            match get_status_use_case.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[network] Failed to get initial status: {e}");
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

        Self { inner, connect_use_case, disconnect_use_case }
    }

    pub fn add_view(&self, view: Box<dyn View<NetworkStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn connect_to_ap(&self, id: String, password: Option<String>) {
        let uc = self.connect_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&id, password.as_deref()).await {
                log::error!("[network] connect_to_ap failed: {e}");
            }
        });
    }

    pub fn disconnect_wifi(&self) {
        let uc = self.disconnect_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute().await {
                log::error!("[network] disconnect_wifi failed: {e}");
            }
        });
    }
}

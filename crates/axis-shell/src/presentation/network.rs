use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::network::connect_to_ap::ConnectToApUseCase;
use axis_application::use_cases::network::disconnect_wifi::DisconnectWifiUseCase;
use axis_domain::models::network::NetworkStatus;
use axis_domain::ports::network::NetworkProvider;
use axis_presentation::{Presenter, View};
use std::sync::Arc;

pub(crate) fn wifi_icon(strength: u8) -> &'static str {
    match strength {
        0..=20 => "network-wireless-signal-none-symbolic",
        21..=40 => "network-wireless-signal-weak-symbolic",
        41..=60 => "network-wireless-signal-ok-symbolic",
        61..=80 => "network-wireless-signal-good-symbolic",
        _ => "network-wireless-signal-excellent-symbolic",
    }
}

pub struct NetworkPresenter {
    inner: Presenter<NetworkStatus>,
    connect_use_case: Arc<ConnectToApUseCase>,
    disconnect_use_case: Arc<DisconnectWifiUseCase>,
}

pub struct NetworkPresenterArgs {
    pub subscribe_uc: Arc<SubscribeUseCase<dyn NetworkProvider, NetworkStatus>>,
    pub get_status_uc: Arc<GetStatusUseCase<dyn NetworkProvider, NetworkStatus>>,
    pub connect_uc: Arc<ConnectToApUseCase>,
    pub disconnect_uc: Arc<DisconnectWifiUseCase>,
}

impl NetworkPresenter {
    pub fn new(args: NetworkPresenterArgs, rt: &tokio::runtime::Runtime) -> Self {
        let NetworkPresenterArgs {
            subscribe_uc,
            get_status_uc,
            connect_uc,
            disconnect_uc,
        } = args;

        let initial_status = rt.block_on(async {
            match get_status_uc.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[network] Failed to get initial status: {e}");
                    Default::default()
                }
            }
        });

        let inner = Presenter::from_subscribe_use_case(subscribe_uc.clone())
            .with_initial_status(initial_status);

        Self {
            inner,
            connect_use_case: connect_uc,
            disconnect_use_case: disconnect_uc,
        }
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

use std::sync::Arc;
use axis_application::use_cases::network::subscribe::SubscribeToNetworkUpdatesUseCase;
use axis_application::use_cases::network::get_status::GetNetworkStatusUseCase;
use axis_application::use_cases::network::scan_wifi::ScanWifiUseCase;
use axis_application::use_cases::network::connect_to_ap::ConnectToApUseCase;
use axis_application::use_cases::network::disconnect_wifi::DisconnectWifiUseCase;
use axis_domain::models::network::NetworkStatus;
use axis_presentation::{Presenter, View};

pub trait NetworkView: View<NetworkStatus> {
    fn on_scan_requested(&self, f: Box<dyn Fn() + 'static>);
    fn on_connect_to_ap(&self, f: Box<dyn Fn(String, Option<String>) + 'static>);
    fn on_disconnect_wifi(&self, f: Box<dyn Fn() + 'static>);
}

impl<T: NetworkView + ?Sized> NetworkView for std::rc::Rc<T> {
    fn on_scan_requested(&self, f: Box<dyn Fn() + 'static>) {
        (**self).on_scan_requested(f);
    }
    fn on_connect_to_ap(&self, f: Box<dyn Fn(String, Option<String>) + 'static>) {
        (**self).on_connect_to_ap(f);
    }
    fn on_disconnect_wifi(&self, f: Box<dyn Fn() + 'static>) {
        (**self).on_disconnect_wifi(f);
    }
}

pub struct NetworkPresenter {
    inner: Presenter<NetworkStatus>,
    scan_use_case: Arc<ScanWifiUseCase>,
    connect_use_case: Arc<ConnectToApUseCase>,
    disconnect_use_case: Arc<DisconnectWifiUseCase>,
}

impl NetworkPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeToNetworkUpdatesUseCase>,
        get_status_use_case: Arc<GetNetworkStatusUseCase>,
        scan_use_case: Arc<ScanWifiUseCase>,
        connect_use_case: Arc<ConnectToApUseCase>,
        disconnect_use_case: Arc<DisconnectWifiUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            get_status_use_case.execute().await.unwrap_or_default()
        });

        let uc = subscribe_use_case.clone();
        let inner = Presenter::new(move || {
            let uc = uc.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = uc.execute().await {
                    while let Some(item) = futures_util::StreamExt::next(&mut stream).await {
                        yield item;
                    }
                }
            })
        }).with_initial_status(initial_status);

        Self { inner, scan_use_case, connect_use_case, disconnect_use_case }
    }

    pub fn add_view(&self, view: Box<dyn View<NetworkStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn scan(&self) {
        let uc = self.scan_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute().await;
        });
    }

    pub fn connect_to_ap(&self, id: String, password: Option<String>) {
        let uc = self.connect_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(&id, password.as_deref()).await;
        });
    }

    pub fn disconnect_wifi(&self) {
        let uc = self.disconnect_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute().await;
        });
    }
}

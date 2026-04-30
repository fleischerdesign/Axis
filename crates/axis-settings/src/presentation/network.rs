use axis_domain::models::network::NetworkStatus;
use axis_domain::ports::network::NetworkProvider;
use axis_presentation::{Presenter, View};
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::network::scan_wifi::ScanWifiUseCase;
use axis_application::use_cases::network::connect_to_ap::ConnectToApUseCase;
use axis_application::use_cases::network::disconnect_wifi::DisconnectWifiUseCase;
use std::sync::Arc;
use std::rc::Rc;

pub trait NetworkView: View<NetworkStatus> {
    #[allow(dead_code)]
    fn on_toggle_wifi(&self, f: Box<dyn Fn(bool) + 'static>);
    fn on_scan_requested(&self, f: Box<dyn Fn() + 'static>);
    fn on_connect(&self, f: Box<dyn Fn(String, String) + 'static>);
    fn on_disconnect(&self, f: Box<dyn Fn() + 'static>);
}

impl<T: NetworkView + ?Sized> NetworkView for Rc<T> {
    fn on_toggle_wifi(&self, f: Box<dyn Fn(bool) + 'static>) {
        (**self).on_toggle_wifi(f);
    }
    fn on_scan_requested(&self, f: Box<dyn Fn() + 'static>) {
        (**self).on_scan_requested(f);
    }
    fn on_connect(&self, f: Box<dyn Fn(String, String) + 'static>) {
        (**self).on_connect(f);
    }
    fn on_disconnect(&self, f: Box<dyn Fn() + 'static>) {
        (**self).on_disconnect(f);
    }
}

pub struct NetworkPresenter {
    inner: Presenter<NetworkStatus>,
    scan_uc: Arc<ScanWifiUseCase>,
    connect_uc: Arc<ConnectToApUseCase>,
    disconnect_uc: Arc<DisconnectWifiUseCase>,
    // We would need a set_enabled use case, but for now we assume the provider handles it or we mock it.
    // Let's assume we need to add a ToggleWifiUseCase if it doesn't exist.
}

impl NetworkPresenter {
    pub fn new(
        subscribe_uc: Arc<SubscribeUseCase<dyn NetworkProvider, NetworkStatus>>,
        get_status_uc: Arc<GetStatusUseCase<dyn NetworkProvider, NetworkStatus>>,
        scan_uc: Arc<ScanWifiUseCase>,
        connect_uc: Arc<ConnectToApUseCase>,
        disconnect_uc: Arc<DisconnectWifiUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            match get_status_uc.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[settings-network] Failed to get initial status: {e}");
                    Default::default()
                }
            }
        });

        let sub = subscribe_uc.clone();
        let inner = Presenter::new(move || {
            let uc = sub.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = uc.execute().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status;
                    }
                }
            })
        }).with_initial_status(initial_status);

        Self {
            inner,
            scan_uc,
            connect_uc,
            disconnect_uc,
        }
    }

    pub async fn bind(&self, view: Box<dyn NetworkView>) {
        let this = self.clone();
        
        view.on_scan_requested(Box::new(move || {
            let uc = this.scan_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute().await {
                    log::error!("[settings-network] scan failed: {e}");
                }
            });
        }));

        let this_c = self.clone();
        view.on_connect(Box::new(move |ssid, password| {
            let uc = this_c.connect_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(&ssid, Some(&password)).await {
                    log::error!("[settings-network] connect failed: {e}");
                }
            });
        }));

        let this_d = self.clone();
        view.on_disconnect(Box::new(move || {
            let uc = this_d.disconnect_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute().await {
                    log::error!("[settings-network] disconnect failed: {e}");
                }
            });
        }));

        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

impl Clone for NetworkPresenter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            scan_uc: self.scan_uc.clone(),
            connect_uc: self.connect_uc.clone(),
            disconnect_uc: self.disconnect_uc.clone(),
        }
    }
}

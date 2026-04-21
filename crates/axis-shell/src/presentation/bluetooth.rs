use std::sync::Arc;
use axis_application::use_cases::bluetooth::subscribe::SubscribeToBluetoothUpdatesUseCase;
use axis_application::use_cases::bluetooth::get_status::GetBluetoothStatusUseCase;
use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::set_powered::SetBluetoothPoweredUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;
use axis_domain::models::bluetooth::BluetoothStatus;
use axis_presentation::{Presenter, View};

pub trait BluetoothView: View<BluetoothStatus> {
    fn on_connect_device(&self, f: Box<dyn Fn(String) + 'static>);
    fn on_disconnect_device(&self, f: Box<dyn Fn(String) + 'static>);
    fn on_set_powered(&self, f: Box<dyn Fn(bool) + 'static>);
    fn on_start_scan(&self, f: Box<dyn Fn() + 'static>);
    fn on_stop_scan(&self, f: Box<dyn Fn() + 'static>);
}

impl<T: BluetoothView + ?Sized> BluetoothView for std::rc::Rc<T> {
    fn on_connect_device(&self, f: Box<dyn Fn(String) + 'static>) {
        (**self).on_connect_device(f);
    }
    fn on_disconnect_device(&self, f: Box<dyn Fn(String) + 'static>) {
        (**self).on_disconnect_device(f);
    }
    fn on_set_powered(&self, f: Box<dyn Fn(bool) + 'static>) {
        (**self).on_set_powered(f);
    }
    fn on_start_scan(&self, f: Box<dyn Fn() + 'static>) {
        (**self).on_start_scan(f);
    }
    fn on_stop_scan(&self, f: Box<dyn Fn() + 'static>) {
        (**self).on_stop_scan(f);
    }
}

pub struct BluetoothPresenter {
    inner: Presenter<BluetoothStatus>,
    connect_use_case: Arc<ConnectBluetoothDeviceUseCase>,
    disconnect_use_case: Arc<DisconnectBluetoothDeviceUseCase>,
    set_powered_use_case: Arc<SetBluetoothPoweredUseCase>,
    start_scan_use_case: Arc<StartBluetoothScanUseCase>,
    stop_scan_use_case: Arc<StopBluetoothScanUseCase>,
}

impl BluetoothPresenter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subscribe_use_case: Arc<SubscribeToBluetoothUpdatesUseCase>,
        get_status_use_case: Arc<GetBluetoothStatusUseCase>,
        connect_use_case: Arc<ConnectBluetoothDeviceUseCase>,
        disconnect_use_case: Arc<DisconnectBluetoothDeviceUseCase>,
        set_powered_use_case: Arc<SetBluetoothPoweredUseCase>,
        start_scan_use_case: Arc<StartBluetoothScanUseCase>,
        stop_scan_use_case: Arc<StopBluetoothScanUseCase>,
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

        Self {
            inner,
            connect_use_case,
            disconnect_use_case,
            set_powered_use_case,
            start_scan_use_case,
            stop_scan_use_case,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<BluetoothStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn connect_device(&self, id: String) {
        let uc = self.connect_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(&id).await;
        });
    }

    pub fn disconnect_device(&self, id: String) {
        let uc = self.disconnect_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(&id).await;
        });
    }

    pub fn set_powered(&self, powered: bool) {
        let uc = self.set_powered_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(powered).await;
        });
    }

    pub fn start_scan(&self) {
        let uc = self.start_scan_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute().await;
        });
    }

    pub fn stop_scan(&self) {
        let uc = self.stop_scan_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute().await;
        });
    }
}

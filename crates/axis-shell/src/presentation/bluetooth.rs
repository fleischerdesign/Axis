use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::models::bluetooth::BluetoothStatus;
use axis_domain::ports::bluetooth::BluetoothProvider;
use axis_presentation::{Presenter, View};
use std::sync::Arc;

pub struct BluetoothPresenter {
    inner: Presenter<BluetoothStatus>,
    connect_use_case: Arc<ConnectBluetoothDeviceUseCase>,
    disconnect_use_case: Arc<DisconnectBluetoothDeviceUseCase>,
    start_scan_use_case: Arc<StartBluetoothScanUseCase>,
    stop_scan_use_case: Arc<StopBluetoothScanUseCase>,
}

pub struct BluetoothPresenterArgs {
    pub subscribe_uc: Arc<SubscribeUseCase<dyn BluetoothProvider, BluetoothStatus>>,
    pub connect_uc: Arc<ConnectBluetoothDeviceUseCase>,
    pub disconnect_uc: Arc<DisconnectBluetoothDeviceUseCase>,
    pub start_scan_uc: Arc<StartBluetoothScanUseCase>,
    pub stop_scan_uc: Arc<StopBluetoothScanUseCase>,
}

impl BluetoothPresenter {
    pub fn new(args: BluetoothPresenterArgs) -> Self {
        let BluetoothPresenterArgs {
            subscribe_uc,
            connect_uc,
            disconnect_uc,
            start_scan_uc,
            stop_scan_uc,
        } = args;

        let inner = Presenter::from_subscribe_use_case(subscribe_uc);

        Self {
            inner,
            connect_use_case: connect_uc,
            disconnect_use_case: disconnect_uc,
            start_scan_use_case: start_scan_uc,
            stop_scan_use_case: stop_scan_uc,
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
            if let Err(e) = uc.execute(&id).await {
                log::error!("[bluetooth] connect_device failed: {e}");
            }
        });
    }

    pub fn disconnect_device(&self, id: String) {
        let uc = self.disconnect_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&id).await {
                log::error!("[bluetooth] disconnect_device failed: {e}");
            }
        });
    }

    pub fn start_scan(&self) {
        let uc = self.start_scan_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute().await {
                log::error!("[bluetooth] start_scan failed: {e}");
            }
        });
    }

    pub fn stop_scan(&self) {
        let uc = self.stop_scan_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute().await {
                log::error!("[bluetooth] stop_scan failed: {e}");
            }
        });
    }
}

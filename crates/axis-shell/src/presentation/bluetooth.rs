use std::sync::Arc;
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;
use axis_domain::models::bluetooth::BluetoothStatus;
use axis_domain::ports::bluetooth::BluetoothProvider;
use axis_presentation::{Presenter, View};

pub struct BluetoothPresenter {
    inner: Presenter<BluetoothStatus>,
    connect_use_case: Arc<ConnectBluetoothDeviceUseCase>,
    disconnect_use_case: Arc<DisconnectBluetoothDeviceUseCase>,
    start_scan_use_case: Arc<StartBluetoothScanUseCase>,
    stop_scan_use_case: Arc<StopBluetoothScanUseCase>,
}

impl BluetoothPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn BluetoothProvider, BluetoothStatus>>,
        get_status_use_case: Arc<GetStatusUseCase<dyn BluetoothProvider, BluetoothStatus>>,
        connect_use_case: Arc<ConnectBluetoothDeviceUseCase>,
        disconnect_use_case: Arc<DisconnectBluetoothDeviceUseCase>,
        start_scan_use_case: Arc<StartBluetoothScanUseCase>,
        stop_scan_use_case: Arc<StopBluetoothScanUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            match get_status_use_case.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[bluetooth] Failed to get initial status: {e}");
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

        Self {
            inner,
            connect_use_case,
            disconnect_use_case,
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

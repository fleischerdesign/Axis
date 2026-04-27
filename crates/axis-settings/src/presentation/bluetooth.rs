use axis_domain::models::bluetooth::BluetoothStatus;
use axis_presentation::{Presenter, View};
use axis_application::use_cases::bluetooth::subscribe::SubscribeToBluetoothUpdatesUseCase;
use axis_application::use_cases::bluetooth::get_status::GetBluetoothStatusUseCase;
use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::set_powered::SetBluetoothPoweredUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;
use std::sync::Arc;
use std::rc::Rc;

pub trait BluetoothView: View<BluetoothStatus> {
    fn on_toggle_power(&self, f: Box<dyn Fn(bool) + 'static>);
    fn on_scan_toggled(&self, f: Box<dyn Fn(bool) + 'static>);
    fn on_device_connect(&self, f: Box<dyn Fn(String) + 'static>);
    fn on_device_disconnect(&self, f: Box<dyn Fn(String) + 'static>);
}

impl<T: BluetoothView + ?Sized> BluetoothView for Rc<T> {
    fn on_toggle_power(&self, f: Box<dyn Fn(bool) + 'static>) {
        (**self).on_toggle_power(f);
    }
    fn on_scan_toggled(&self, f: Box<dyn Fn(bool) + 'static>) {
        (**self).on_scan_toggled(f);
    }
    fn on_device_connect(&self, f: Box<dyn Fn(String) + 'static>) {
        (**self).on_device_connect(f);
    }
    fn on_device_disconnect(&self, f: Box<dyn Fn(String) + 'static>) {
        (**self).on_device_disconnect(f);
    }
}

pub struct BluetoothPresenter {
    inner: Presenter<BluetoothStatus>,
    connect_uc: Arc<ConnectBluetoothDeviceUseCase>,
    disconnect_uc: Arc<DisconnectBluetoothDeviceUseCase>,
    set_powered_uc: Arc<SetBluetoothPoweredUseCase>,
    start_scan_uc: Arc<StartBluetoothScanUseCase>,
    stop_scan_uc: Arc<StopBluetoothScanUseCase>,
}

impl BluetoothPresenter {
    pub fn new(
        subscribe_uc: Arc<SubscribeToBluetoothUpdatesUseCase>,
        get_status_uc: Arc<GetBluetoothStatusUseCase>,
        connect_uc: Arc<ConnectBluetoothDeviceUseCase>,
        disconnect_uc: Arc<DisconnectBluetoothDeviceUseCase>,
        set_powered_uc: Arc<SetBluetoothPoweredUseCase>,
        start_scan_uc: Arc<StartBluetoothScanUseCase>,
        stop_scan_uc: Arc<StopBluetoothScanUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            get_status_uc.execute().await.unwrap_or_default()
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
            connect_uc,
            disconnect_uc,
            set_powered_uc,
            start_scan_uc,
            stop_scan_uc,
        }
    }

    pub async fn bind(&self, view: Box<dyn BluetoothView>) {
        let this = self.clone();
        
        view.on_toggle_power(Box::new(move |powered| {
            let uc = this.set_powered_uc.clone();
            tokio::spawn(async move {
                let _ = uc.execute(powered).await;
            });
        }));

        let this_scan = self.clone();
        view.on_scan_toggled(Box::new(move |scanning| {
            let start = this_scan.start_scan_uc.clone();
            let stop = this_scan.stop_scan_uc.clone();
            tokio::spawn(async move {
                if scanning {
                    let _ = start.execute().await;
                } else {
                    let _ = stop.execute().await;
                }
            });
        }));

        let this_c = self.clone();
        view.on_device_connect(Box::new(move |id| {
            let uc = this_c.connect_uc.clone();
            tokio::spawn(async move {
                let _ = uc.execute(&id).await;
            });
        }));

        let this_d = self.clone();
        view.on_device_disconnect(Box::new(move |id| {
            let uc = this_d.disconnect_uc.clone();
            tokio::spawn(async move {
                let _ = uc.execute(&id).await;
            });
        }));

        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

impl Clone for BluetoothPresenter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            connect_uc: self.connect_uc.clone(),
            disconnect_uc: self.disconnect_uc.clone(),
            set_powered_uc: self.set_powered_uc.clone(),
            start_scan_uc: self.start_scan_uc.clone(),
            stop_scan_uc: self.stop_scan_uc.clone(),
        }
    }
}

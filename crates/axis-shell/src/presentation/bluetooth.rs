use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_application::use_cases::notifications::show_notification::ShowNotificationUseCase;
use axis_domain::models::bluetooth::BluetoothStatus;
use axis_domain::models::notifications::{Notification, Urgency};
use axis_domain::ports::bluetooth::BluetoothProvider;
use axis_presentation::{Presenter, View};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) fn bluetooth_icon(status: &BluetoothStatus) -> &'static str {
    let has_connected = status.devices.iter().any(|d| d.connected);
    if has_connected {
        "bluetooth-active-symbolic"
    } else {
        "bluetooth-symbolic"
    }
}

pub struct BluetoothPresenter {
    inner: Presenter<BluetoothStatus>,
    connect_use_case: Arc<ConnectBluetoothDeviceUseCase>,
    disconnect_use_case: Arc<DisconnectBluetoothDeviceUseCase>,
    start_scan_use_case: Arc<StartBluetoothScanUseCase>,
    stop_scan_use_case: Arc<StopBluetoothScanUseCase>,
    show_notification_use_case: Option<Arc<ShowNotificationUseCase>>,
}

pub struct BluetoothPresenterArgs {
    pub subscribe_uc: Arc<SubscribeUseCase<dyn BluetoothProvider, BluetoothStatus>>,
    pub connect_uc: Arc<ConnectBluetoothDeviceUseCase>,
    pub disconnect_uc: Arc<DisconnectBluetoothDeviceUseCase>,
    pub start_scan_uc: Arc<StartBluetoothScanUseCase>,
    pub stop_scan_uc: Arc<StopBluetoothScanUseCase>,
    pub show_notification_uc: Option<Arc<ShowNotificationUseCase>>,
}

impl BluetoothPresenter {
    pub fn new(args: BluetoothPresenterArgs) -> Self {
        let BluetoothPresenterArgs {
            subscribe_uc,
            connect_uc,
            disconnect_uc,
            start_scan_uc,
            stop_scan_uc,
            show_notification_uc,
        } = args;

        let inner = Presenter::from_subscribe_use_case(subscribe_uc);

        Self {
            inner,
            connect_use_case: connect_uc,
            disconnect_use_case: disconnect_uc,
            start_scan_use_case: start_scan_uc,
            stop_scan_use_case: stop_scan_uc,
            show_notification_use_case: show_notification_uc,
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
        let notif_uc = self.show_notification_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&id).await {
                log::error!("[bluetooth] connect_device failed: {e}");
                if let Some(show_notif) = notif_uc {
                    let notification = Notification {
                        id: 0,
                        app_name: "Bluetooth".to_string(),
                        app_icon: "bluetooth-symbolic".to_string(),
                        summary: "Connection Failed".to_string(),
                        body: format!("Could not connect to device: {e}"),
                        urgency: Urgency::Normal,
                        actions: vec![],
                        timeout: 5000,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                        internal_id: 0,
                        ignore_dnd: false,
                        input_placeholder: None,
                    };
                    let _ = show_notif.execute(notification, HashMap::new()).await;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use axis_domain::models::bluetooth::BluetoothDevice;

    #[test]
    fn test_bluetooth_icon_no_connected_devices() {
        let status = BluetoothStatus {
            powered: true,
            is_scanning: false,
            devices: vec![BluetoothDevice {
                connected: false,
                ..Default::default()
            }],
            pending_pairing: None,
        };
        assert_eq!(bluetooth_icon(&status), "bluetooth-symbolic");
    }

    #[test]
    fn test_bluetooth_icon_with_connected_device() {
        let status = BluetoothStatus {
            powered: true,
            is_scanning: false,
            devices: vec![
                BluetoothDevice {
                    connected: false,
                    ..Default::default()
                },
                BluetoothDevice {
                    connected: true,
                    ..Default::default()
                },
            ],
            pending_pairing: None,
        };
        assert_eq!(bluetooth_icon(&status), "bluetooth-active-symbolic");
    }
}

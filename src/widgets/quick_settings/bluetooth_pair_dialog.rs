use crate::services::bluetooth::{self, BluetoothCmd, PairingRequest, PairingType};
use crate::services::notifications::{Notification, NotificationAction};
use async_channel::Sender;
use std::collections::HashMap;
use std::sync::Arc;

const BLUETOOTH_PAIRING_NOTIF_ID: u32 = 4294967295;

fn create_pairing_notification(req: &PairingRequest, tx: Sender<BluetoothCmd>) -> Notification {
    let mut on_action: HashMap<String, Arc<dyn Fn() + Send + Sync>> = HashMap::new();

    on_action.insert(
        "accept".to_string(),
        Arc::new({
            let tx = tx.clone();
            move || {
                let _ = tx.try_send(BluetoothCmd::PairAccept);
            }
        }),
    );

    on_action.insert(
        "reject".to_string(),
        Arc::new({
            move || {
                let _ = tx.try_send(BluetoothCmd::PairReject);
            }
        }),
    );

    let body = match req.pairing_type {
        PairingType::Confirmation => req
            .passkey
            .as_ref()
            .map(|pk| {
                format!("PIN: {pk}\nBestätigen Sie, dass der PIN auf dem Gerät übereinstimmt.")
            })
            .unwrap_or_else(|| "Bestätigen Sie die Kopplung.".to_string()),
        PairingType::PinCode => {
            "Geben Sie den PIN-Code ein, der am Gerät angezeigt wird.".to_string()
        }
        PairingType::Passkey => "Geben Sie den Passkey ein.".to_string(),
        PairingType::Authorization => "Möchten Sie die Kopplung erlauben?".to_string(),
    };

    Notification {
        id: BLUETOOTH_PAIRING_NOTIF_ID,
        app_name: "Bluetooth".to_string(),
        app_icon: "bluetooth-active-symbolic".to_string(),
        summary: req.device_name.clone(),
        body,
        urgency: 2,
        timestamp: chrono::Local::now().timestamp(),
        actions: vec![
            NotificationAction {
                key: "accept".to_string(),
                label: "Bestätigen".to_string(),
            },
            NotificationAction {
                key: "reject".to_string(),
                label: "Ablehnen".to_string(),
            },
        ],
        on_action: Some(on_action),
        internal_id: 1,
    }
}

pub fn send_pairing_notification(
    req: &PairingRequest,
    tx: Sender<BluetoothCmd>,
    raw_tx: &async_channel::Sender<Notification>,
) {
    let notification = create_pairing_notification(req, tx);
    bluetooth::set_pairing_notification_id(BLUETOOTH_PAIRING_NOTIF_ID);
    let _ = raw_tx.try_send(notification);
}

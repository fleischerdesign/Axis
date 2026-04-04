use axis_core::services::continuity::ContinuityCmd;
use crate::services::notifications::{Notification, NotificationAction};
use crate::services::notifications::server::NotificationCmd;
use async_channel::Sender;
use std::collections::HashMap;
use std::sync::Arc;

const CONTINUITY_PAIRING_NOTIF_ID: u32 = 4294967294;
const CONTINUITY_CONNECTED_NOTIF_ID: u32 = 4294967293;

fn create_pairing_action_handlers(
    tx: &Sender<ContinuityCmd>,
) -> HashMap<String, Arc<dyn Fn() + Send + Sync>> {
    let mut on_action: HashMap<String, Arc<dyn Fn() + Send + Sync>> = HashMap::new();

    on_action.insert(
        "accept".to_string(),
        Arc::new({
            let tx = tx.clone();
            move || {
                let _ = tx.try_send(ContinuityCmd::ConfirmPin);
            }
        }),
    );

    on_action.insert(
        "reject".to_string(),
        Arc::new({
            let tx = tx.clone();
            move || {
                let _ = tx.try_send(ContinuityCmd::RejectPin);
            }
        }),
    );

    on_action
}

fn create_pairing_notification(
    peer_name: &str,
    pin: &str,
    is_incoming: bool,
    tx: Sender<ContinuityCmd>,
) -> Notification {
    let body = if is_incoming {
        format!("Kopplungsanfrage von {}\nPIN: {}", peer_name, pin)
    } else {
        format!(
            "Bitte bestätigen Sie die PIN {} auf dem Gerät {}",
            pin, peer_name
        )
    };

    Notification {
        id: CONTINUITY_PAIRING_NOTIF_ID,
        app_name: "Continuity".to_string(),
        app_icon: "computer-symbolic".to_string(),
        summary: "Gerätekopplung".to_string(),
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
        on_action: Some(create_pairing_action_handlers(&tx)),
        internal_id: 2,
    }
}

fn create_connected_notification(peer_name: &str) -> Notification {
    Notification {
        id: CONTINUITY_CONNECTED_NOTIF_ID,
        app_name: "Continuity".to_string(),
        app_icon: "computer-symbolic".to_string(),
        summary: "Verbunden".to_string(),
        body: format!("Verbunden mit {}", peer_name),
        urgency: 1,
        timestamp: chrono::Local::now().timestamp(),
        actions: vec![],
        on_action: None,
        internal_id: 3,
    }
}

pub fn send_pairing_notification(
    peer_name: &str,
    pin: &str,
    is_incoming: bool,
    tx: Sender<ContinuityCmd>,
    cmd_tx: &Sender<NotificationCmd>,
) {
    let notification = create_pairing_notification(peer_name, pin, is_incoming, tx);
    let _ = cmd_tx.try_send(NotificationCmd::Show(notification));
}

pub fn send_connected_notification(peer_name: &str, cmd_tx: &Sender<NotificationCmd>) {
    let notification = create_connected_notification(peer_name);
    let _ = cmd_tx.try_send(NotificationCmd::Show(notification));
}

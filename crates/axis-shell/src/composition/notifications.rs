use super::providers::Providers;
use super::use_cases::UseCases;
use crate::i18n::de;
use axis_domain::models::config::AxisConfig;
use std::collections::HashMap;
use std::sync::Arc;

pub fn setup(providers: &Providers, uc: &UseCases, rt: &tokio::runtime::Runtime) {
    subscribe_continuity_notifications(
        providers.continuity.clone(),
        uc.show_notification.clone(),
        uc.continuity_confirm_pin.clone(),
        uc.continuity_reject_pin.clone(),
        rt,
    );

    subscribe_bluetooth_pairing_notifications(
        providers.bluetooth.clone(),
        uc.show_notification.clone(),
        uc.bt_pair_accept.clone(),
        uc.bt_pair_reject.clone(),
    );

    wire_continuity_sync(providers.config.clone(), providers.continuity.clone(), rt);
}

fn subscribe_continuity_notifications(
    continuity_provider: Arc<dyn axis_domain::ports::continuity::ContinuityProvider>,
    show_notification_uc: Arc<
        axis_application::use_cases::notifications::show_notification::ShowNotificationUseCase,
    >,
    confirm_pin_uc: Arc<axis_application::use_cases::continuity::confirm_pin::ConfirmPinUseCase>,
    reject_pin_uc: Arc<axis_application::use_cases::continuity::reject_pin::RejectPinUseCase>,
    rt: &tokio::runtime::Runtime,
) {
    rt.spawn(async move {
        let mut stream = match continuity_provider.subscribe().await {
            Ok(s) => s,
            Err(e) => {
                log::error!("[continuity:notifications] Failed to subscribe: {e}");
                return;
            }
        };

        let mut last_notified: Option<String> = None;
        let mut last_connected: Option<String> = None;

        while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
            if let Some(pending) = &status.pending_pin {
                if pending.is_incoming {
                    let peer_id = &pending.peer_id;
                    if last_notified.as_deref() != Some(peer_id) {
                        last_notified = Some(peer_id.clone());

                        let notification = axis_domain::models::notifications::Notification {
                            id: u32::MAX - 2,
                            app_name: "Continuity".to_string(),
                            app_icon: "computer-symbolic".to_string(),
                            summary: de::PAIRING_TITLE.to_string(),
                            body: format!(
                                "Kopplungsanfrage von {}\nPIN: {}",
                                pending.peer_name, pending.pin
                            ),
                            urgency: axis_domain::models::notifications::Urgency::Critical,
                            actions: vec![
                                axis_domain::models::notifications::NotificationAction {
                                    key: "accept".into(),
                                    label: de::CONFIRM.into(),
                                },
                                axis_domain::models::notifications::NotificationAction {
                                    key: "reject".into(),
                                    label: de::REJECT.into(),
                                },
                            ],
                            timeout: 0,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64,
                            internal_id: 0,
                            ignore_dnd: true,
                            input_placeholder: None,
                        };

                        let mut action_handlers: HashMap<
                            String,
                            axis_domain::ports::notifications::ActionHandler,
                        > = HashMap::new();

                        action_handlers.insert(
                            "accept".into(),
                            Arc::new({
                                let uc = confirm_pin_uc.clone();
                                move |_: Option<String>| {
                                    let uc = uc.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = uc.execute().await {
                                            log::error!(
                                                "[continuity:notifications] confirm_pin failed: {e}"
                                            );
                                        }
                                    });
                                }
                            }),
                        );

                        action_handlers.insert(
                            "reject".into(),
                            Arc::new({
                                let uc = reject_pin_uc.clone();
                                move |_: Option<String>| {
                                    let uc = uc.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = uc.execute().await {
                                            log::error!(
                                                "[continuity:notifications] reject_pin failed: {e}"
                                            );
                                        }
                                    });
                                }
                            }),
                        );

                        if let Err(e) = show_notification_uc
                            .execute(notification, action_handlers)
                            .await
                        {
                            log::error!(
                                "[continuity:notifications] show pairing notification failed: {e}"
                            );
                        }
                    }
                }
            } else {
                last_notified = None;
            }

            if let Some(conn) = &status.active_connection {
                if status
                    .peer_configs
                    .get(&conn.peer_id)
                    .is_some_and(|c| c.trusted)
                {
                    if last_connected.as_deref() != Some(&conn.peer_id) {
                        last_connected = Some(conn.peer_id.clone());

                        let notification = axis_domain::models::notifications::Notification {
                            id: u32::MAX - 3,
                            app_name: "Continuity".to_string(),
                            app_icon: "computer-symbolic".to_string(),
                            summary: de::CONNECTED.to_string(),
                            body: format!(
                                "{} {}",
                                de::CONNECTED_TO.replace("{}", ""),
                                conn.peer_name
                            ),
                            urgency: axis_domain::models::notifications::Urgency::Normal,
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

                        if let Err(e) = show_notification_uc
                            .execute(notification, HashMap::new())
                            .await
                        {
                            log::error!(
                                "[continuity:notifications] show connected notification failed: {e}"
                            );
                        }
                    }
                } else {
                    last_connected = None;
                }
            } else {
                last_connected = None;
            }
        }
    });
}

fn wire_continuity_sync(
    config_provider: Arc<dyn axis_domain::ports::config::ConfigProvider>,
    continuity_provider: Arc<dyn axis_domain::ports::continuity::ContinuityProvider>,
    rt: &tokio::runtime::Runtime,
) {
    let initial_enabled = config_provider
        .get()
        .map(|c| c.continuity.enabled)
        .unwrap_or(false);

    {
        let cont = continuity_provider.clone();
        let mut config_stream = match config_provider.subscribe() {
            Ok(s) => s,
            Err(e) => {
                log::error!("[continuity:sync] config subscribe failed: {e}");
                return;
            }
        };
        let mut last_enabled = Some(initial_enabled);
        rt.spawn(async move {
            if initial_enabled
                && let Err(e) = cont.set_enabled(true).await
            {
                log::error!("[continuity:sync] initial config→continuity failed: {e}");
            }
            while let Some(config) = futures_util::StreamExt::next(&mut config_stream).await {
                let enabled = config.continuity.enabled;
                if last_enabled != Some(enabled) {
                    last_enabled = Some(enabled);
                    if let Err(e) = cont.set_enabled(enabled).await {
                        log::error!("[continuity:sync] config→continuity failed: {e}");
                    }
                }
            }
        });
    }

    {
        let cfg = config_provider.clone();
        let mut cont_stream = match rt.block_on(continuity_provider.subscribe()) {
            Ok(s) => s,
            Err(e) => {
                log::error!("[continuity:sync] continuity subscribe failed: {e}");
                return;
            }
        };
        let mut last_enabled = Some(initial_enabled);
        rt.spawn(async move {
            while let Some(status) = futures_util::StreamExt::next(&mut cont_stream).await {
                let enabled = status.enabled;
                if last_enabled != Some(enabled) {
                    last_enabled = Some(enabled);
                    if let Err(e) = cfg.update(Box::new(move |c: &mut AxisConfig| {
                        c.continuity.enabled = enabled;
                    })) {
                        log::error!("[continuity:sync] continuity→config failed: {e}");
                    }
                }
            }
        });
    }
}

fn subscribe_bluetooth_pairing_notifications(
    bluetooth_provider: Arc<dyn axis_domain::ports::bluetooth::BluetoothProvider>,
    show_notification_uc: Arc<
        axis_application::use_cases::notifications::show_notification::ShowNotificationUseCase,
    >,
    pair_accept_uc: Arc<axis_application::use_cases::bluetooth::pair_accept::PairAcceptUseCase>,
    pair_reject_uc: Arc<axis_application::use_cases::bluetooth::pair_reject::PairRejectUseCase>,
) {
    tokio::spawn(async move {
        let mut stream = match bluetooth_provider.subscribe().await {
            Ok(s) => s,
            Err(e) => {
                log::error!("[bluetooth:notifications] Failed to subscribe: {e}");
                return;
            }
        };

        let mut last_notified: Option<String> = None;

        while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
            if let Some(pairing) = &status.pending_pairing {
                let device_path = &pairing.device_path;
                if last_notified.as_deref() != Some(device_path) {
                    last_notified = Some(device_path.clone());

                    let (body, input_placeholder) = match pairing.pairing_type {
                        axis_domain::models::bluetooth::PairingType::Confirmation => {
                            let msg = pairing
                                .passkey
                                .as_ref()
                                .map(|pk| {
                                    format!("{} {}", de::PIN_CODE_INPUT.replace("...", ""), pk)
                                })
                                .unwrap_or_else(|| de::PAIRING_CONFIRM.to_string());
                            (msg, None)
                        }
                        axis_domain::models::bluetooth::PairingType::PinCode => (
                            de::PIN_CODE_INPUT.to_string(),
                            Some(de::PAIRING_PIN_PROMPT.to_string()),
                        ),
                        axis_domain::models::bluetooth::PairingType::Passkey => (
                            de::PASSKEY_INPUT.to_string(),
                            Some(de::PAIRING_PASSKEY_PROMPT.to_string()),
                        ),
                        axis_domain::models::bluetooth::PairingType::Authorization => {
                            (de::PAIRING_AUTHORIZATION.to_string(), None)
                        }
                    };

                    let notification = axis_domain::models::notifications::Notification {
                        id: u32::MAX,
                        app_name: "Bluetooth".to_string(),
                        app_icon: "bluetooth-active-symbolic".to_string(),
                        summary: pairing.device_name.clone(),
                        body,
                        urgency: axis_domain::models::notifications::Urgency::Critical,
                        actions: vec![
                            axis_domain::models::notifications::NotificationAction {
                                key: "accept".into(),
                                label: de::CONFIRM.into(),
                            },
                            axis_domain::models::notifications::NotificationAction {
                                key: "reject".into(),
                                label: de::REJECT.into(),
                            },
                        ],
                        timeout: 0,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                        internal_id: 0,
                        ignore_dnd: true,
                        input_placeholder,
                    };

                    let mut action_handlers: HashMap<
                        String,
                        axis_domain::ports::notifications::ActionHandler,
                    > = HashMap::new();

                    action_handlers.insert(
                        "accept".into(),
                        Arc::new({
                            let uc = pair_accept_uc.clone();
                            move |input: Option<String>| {
                                let uc = uc.clone();
                                let value = input.map(|s| s.into_bytes()).unwrap_or_default();
                                tokio::spawn(async move {
                                    if let Err(e) = uc.execute(value).await {
                                        log::error!(
                                            "[bluetooth:notifications] pair_accept failed: {e}"
                                        );
                                    }
                                });
                            }
                        }),
                    );

                    action_handlers.insert(
                        "reject".into(),
                        Arc::new({
                            let uc = pair_reject_uc.clone();
                            move |_: Option<String>| {
                                let uc = uc.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = uc.execute().await {
                                        log::error!(
                                            "[bluetooth:notifications] pair_reject failed: {e}"
                                        );
                                    }
                                });
                            }
                        }),
                    );

                    if let Err(e) = show_notification_uc
                        .execute(notification, action_handlers)
                        .await
                    {
                        log::error!(
                            "[bluetooth:notifications] show pairing notification failed: {e}"
                        );
                    }
                }
            } else {
                last_notified = None;
            }
        }
    });
}

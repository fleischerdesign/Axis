use axis_domain::models::continuity::{
    ContinuityStatus, InputEvent, PeerArrangement, PeerConfig, Side,
};
use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider, ContinuityStream};
use async_trait::async_trait;
use log::{error, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use zbus::Connection;

#[derive(Clone)]
struct CachedState {
    inner: Arc<Mutex<ContinuityStatus>>,
}

impl CachedState {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ContinuityStatus::default())),
        }
    }

    fn get(&self) -> ContinuityStatus {
        self.inner.lock().unwrap().clone()
    }

    fn set(&self, status: ContinuityStatus) {
        *self.inner.lock().unwrap() = status;
    }
}

pub struct ContinuityDbusProxy {
    cached: CachedState,
    status_tx: watch::Sender<ContinuityStatus>,
}

impl ContinuityDbusProxy {
    pub fn new() -> Arc<Self> {
        let (status_tx, _) = watch::channel(ContinuityStatus::default());
        Arc::new(Self {
            cached: CachedState::new(),
            status_tx,
        })
    }

    pub async fn init(self: &Arc<Self>) -> Result<(), ContinuityError> {
        let conn = Connection::session()
            .await
            .map_err(|e| ContinuityError::ProviderError(format!("D-Bus connect: {e}")))?;

        let initial_state = self.call_get_state(&conn).await?;
        self.cached.set(initial_state.clone());
        let _ = self.status_tx.send(initial_state);

        self.start_signal_listener(conn).await?;

        Ok(())
    }

    async fn call_get_state(
        &self,
        conn: &Connection,
    ) -> Result<ContinuityStatus, ContinuityError> {
        let result: String = conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Continuity",
                Some("org.axis.Shell.Continuity"),
                "GetState",
                &(),
            )
            .await
            .map_err(|e| ContinuityError::ProviderError(format!("GetState: {e}")))?
            .body()
            .deserialize()
            .map_err(|e| ContinuityError::ProviderError(format!("GetState body: {e}")))?;

        serde_json::from_str(&result)
            .map_err(|e| ContinuityError::ProviderError(format!("GetState parse: {e}")))
    }

    async fn call_method_str(&self, method: &str, arg: &str) -> Result<(), ContinuityError> {
        let conn = Connection::session()
            .await
            .map_err(|e| ContinuityError::ProviderError(format!("D-Bus connect: {e}")))?;

        let result: bool = conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Continuity",
                Some("org.axis.Shell.Continuity"),
                method,
                &(arg),
            )
            .await
            .map_err(|e| ContinuityError::ProviderError(format!("{method}: {e}")))?
            .body()
            .deserialize()
            .map_err(|e| ContinuityError::ProviderError(format!("{method} body: {e}")))?;

        if result {
            Ok(())
        } else {
            Err(ContinuityError::ProviderError(format!("{method} returned false")))
        }
    }

    async fn call_method_bool(&self, method: &str, arg: bool) -> Result<(), ContinuityError> {
        let conn = Connection::session()
            .await
            .map_err(|e| ContinuityError::ProviderError(format!("D-Bus connect: {e}")))?;

        let result: bool = conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Continuity",
                Some("org.axis.Shell.Continuity"),
                method,
                &(arg),
            )
            .await
            .map_err(|e| ContinuityError::ProviderError(format!("{method}: {e}")))?
            .body()
            .deserialize()
            .map_err(|e| ContinuityError::ProviderError(format!("{method} body: {e}")))?;

        if result {
            Ok(())
        } else {
            Err(ContinuityError::ProviderError(format!("{method} returned false")))
        }
    }

    async fn call_method_empty(&self, method: &str) -> Result<(), ContinuityError> {
        let conn = Connection::session()
            .await
            .map_err(|e| ContinuityError::ProviderError(format!("D-Bus connect: {e}")))?;

        let result: bool = conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Continuity",
                Some("org.axis.Shell.Continuity"),
                method,
                &(),
            )
            .await
            .map_err(|e| ContinuityError::ProviderError(format!("{method}: {e}")))?
            .body()
            .deserialize()
            .map_err(|e| ContinuityError::ProviderError(format!("{method} body: {e}")))?;

        if result {
            Ok(())
        } else {
            Err(ContinuityError::ProviderError(format!("{method} returned false")))
        }
    }

    async fn start_signal_listener(
        self: &Arc<Self>,
        conn: Connection,
    ) -> Result<(), ContinuityError> {
        let proxy = zbus::Proxy::new(
            &conn,
            "org.axis.Shell",
            "/org/axis/Shell/Continuity",
            "org.axis.Shell.Continuity",
        )
        .await
        .map_err(|e| ContinuityError::ProviderError(format!("signal proxy: {e}")))?;

        let this = self.clone();
        tokio::spawn(async move {
            let mut signal = match proxy.receive_signal("StateChanged").await {
                Ok(s) => s,
                Err(e) => {
                    error!("[continuity-proxy] StateChanged subscribe failed: {e}");
                    return;
                }
            };

            use futures_util::StreamExt;
            while let Some(msg) = signal.next().await {
                let body = msg.body();
                let json: Result<(String,), _> = body.deserialize();
                let (json,) = match json {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("[continuity-proxy] Failed to parse signal: {e}");
                        continue;
                    }
                };

                match serde_json::from_str::<ContinuityStatus>(&json) {
                    Ok(status) => {
                        this.cached.set(status.clone());
                        let _ = this.status_tx.send(status);
                    }
                    Err(e) => {
                        warn!("[continuity-proxy] Failed to parse state: {e}");
                    }
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl ContinuityProvider for ContinuityDbusProxy {
    async fn get_status(&self) -> Result<ContinuityStatus, ContinuityError> {
        Ok(self.cached.get())
    }

    async fn subscribe(&self) -> Result<ContinuityStream, ContinuityError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), ContinuityError> {
        self.call_method_bool("SetEnabled", enabled).await
    }

    async fn connect_to_peer(&self, peer_id: &str) -> Result<(), ContinuityError> {
        self.call_method_str("ConnectToPeer", peer_id).await
    }

    async fn confirm_pin(&self) -> Result<(), ContinuityError> {
        self.call_method_empty("ConfirmPin").await
    }

    async fn reject_pin(&self) -> Result<(), ContinuityError> {
        self.call_method_empty("RejectPin").await
    }

    async fn disconnect(&self) -> Result<(), ContinuityError> {
        self.call_method_empty("Disconnect").await
    }

    async fn cancel_reconnect(&self) -> Result<(), ContinuityError> {
        self.call_method_empty("CancelReconnect").await
    }

    async fn unpair(&self, peer_id: &str) -> Result<(), ContinuityError> {
        self.call_method_str("Unpair", peer_id).await
    }

    async fn start_sharing(&self, _side: Side, _edge_pos: f64) -> Result<(), ContinuityError> {
        Err(ContinuityError::ProviderError("start_sharing not supported via D-Bus".into()))
    }

    async fn stop_sharing(&self, _edge_pos: f64) -> Result<(), ContinuityError> {
        Err(ContinuityError::ProviderError("stop_sharing not supported via D-Bus".into()))
    }

    async fn send_input(&self, _event: InputEvent) -> Result<(), ContinuityError> {
        Err(ContinuityError::ProviderError("send_input not supported via D-Bus".into()))
    }

    async fn force_local(&self) -> Result<(), ContinuityError> {
        Err(ContinuityError::ProviderError("force_local not supported via D-Bus".into()))
    }

    async fn set_peer_arrangement(&self, arrangement: PeerArrangement) -> Result<(), ContinuityError> {
        let json = serde_json::to_string(&arrangement)
            .map_err(|e| ContinuityError::ProviderError(format!("serialize: {e}")))?;
        self.call_method_str("SetPeerArrangement", &json).await
    }

    async fn update_peer_configs(
        &self,
        configs: HashMap<String, PeerConfig>,
    ) -> Result<(), ContinuityError> {
        let json = serde_json::to_string(&configs)
            .map_err(|e| ContinuityError::ProviderError(format!("serialize: {e}")))?;
        self.call_method_str("UpdatePeerConfigs", &json).await
    }
}

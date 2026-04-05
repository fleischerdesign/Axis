use std::cell::RefCell;
use std::rc::Rc;
use gtk4::glib;
use zbus::Connection;

use axis_core::services::continuity::dbus::ContinuityStateSnapshot;
use axis_core::services::continuity::{PeerArrangement, PeerConfig};

macro_rules! dbus_command {
    ($fn_name:ident, $method:ident) => {
        pub async fn $fn_name(&self) -> Result<bool, Box<dyn std::error::Error>> {
            let result: bool = self.conn
                .call_method(
                    Some("org.axis.Shell"),
                    "/org/axis/Shell/Continuity",
                    Some("org.axis.Shell.Continuity"),
                    stringify!($method),
                    &(),
                )
                .await?
                .body()
                .deserialize()?;
            Ok(result)
        }
    };
    ($fn_name:ident, $method:ident, $param_name:ident: $param_ty:ty) => {
        pub async fn $fn_name(&self, $param_name: $param_ty) -> Result<bool, Box<dyn std::error::Error>> {
            let result: bool = self.conn
                .call_method(
                    Some("org.axis.Shell"),
                    "/org/axis/Shell/Continuity",
                    Some("org.axis.Shell.Continuity"),
                    stringify!($method),
                    &($param_name,),
                )
                .await?
                .body()
                .deserialize()?;
            Ok(result)
        }
    };
}

/// Typed D-Bus client for org.axis.Shell.Continuity.
/// Subscribes to StateChanged signal and notifies listeners.
pub struct ContinuityProxy {
    conn: Connection,
    cached: Rc<RefCell<ContinuityStateSnapshot>>,
    listeners: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
}

impl ContinuityProxy {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::session().await?;

        // Verify the Continuity D-Bus interface is available
        let _ = Self::call_get_state(&conn).await?;

        let proxy = Self {
            conn,
            cached: Rc::new(RefCell::new(ContinuityStateSnapshot::default())),
            listeners: Rc::new(RefCell::new(Vec::new())),
        };

        // 1. Start signal listener FIRST — catches any changes from now on
        proxy.start_signal_listener();

        // 2. Then load current state — fills the gap
        if let Ok(state) = Self::call_get_state(&proxy.conn).await {
            *proxy.cached.borrow_mut() = state;
            Self::notify_listeners(&proxy.listeners);
        }

        Ok(proxy)
    }

    pub fn state(&self) -> ContinuityStateSnapshot {
        self.cached.borrow().clone()
    }

    pub fn on_change(&self, f: impl Fn() + 'static) {
        f();
        self.listeners.borrow_mut().push(Box::new(f));
    }

    /// Re-fetch state from D-Bus and notify listeners.
    /// Use this when the page becomes visible to ensure fresh data.
    pub fn reload(&self) {
        let cached = self.cached.clone();
        let listeners = self.listeners.clone();
        let conn = self.conn.clone();

        glib::spawn_future_local(async move {
            if let Ok(state) = Self::call_get_state(&conn).await {
                let changed = *cached.borrow() != state;
                *cached.borrow_mut() = state;
                if changed {
                    Self::notify_listeners(&listeners);
                }
            }
        });
    }

    // ── Signal Listener ─────────────────────────────────────────────────

    fn start_signal_listener(&self) {
        use futures_util::StreamExt;
        let conn = self.conn.clone();
        let cached = self.cached.clone();
        let listeners = self.listeners.clone();

        let _handle = glib::spawn_future_local(async move {
            let proxy = match zbus::Proxy::new(
                &conn,
                "org.axis.Shell",
                "/org/axis/Shell/Continuity",
                "org.axis.Shell.Continuity",
            )
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("[continuity-proxy] Failed to create signal proxy: {e}");
                    return;
                }
            };

            let mut stream = match proxy.receive_signal("StateChanged").await {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("[continuity-proxy] Failed to subscribe to StateChanged: {e}");
                    return;
                }
            };

            while let Some(signal) = stream.next().await {
                let body = signal.body();
                match body.deserialize::<(String,)>() {
                    Ok((json,)) => {
                        if let Ok(state) = serde_json::from_str::<ContinuityStateSnapshot>(&json) {
                            *cached.borrow_mut() = state;
                            Self::notify_listeners(&listeners);
                        }
                    }
                    Err(e) => {
                        log::warn!("[continuity-proxy] Failed to parse signal: {e}");
                    }
                }
            }
        });
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn notify_listeners(listeners: &Rc<RefCell<Vec<Box<dyn Fn()>>>>) {
        let taken = std::mem::take(&mut *listeners.borrow_mut());
        for listener in &taken {
            listener();
        }
        *listeners.borrow_mut() = taken;
    }

    async fn call_get_state(
        conn: &Connection,
    ) -> Result<ContinuityStateSnapshot, Box<dyn std::error::Error>> {
        let json: String = conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Continuity",
                Some("org.axis.Shell.Continuity"),
                "GetState",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(serde_json::from_str(&json)?)
    }

    // ── Commands ────────────────────────────────────────────────────────

    dbus_command!(connect_to_peer, ConnectToPeer, peer_id: &str);
    dbus_command!(confirm_pin, ConfirmPin);
    dbus_command!(reject_pin, RejectPin);
    dbus_command!(disconnect, Disconnect);
    dbus_command!(set_enabled, SetEnabled, enabled: bool);
    dbus_command!(unpair, Unpair, peer_id: &str);

    pub async fn set_peer_arrangement(
        &self,
        arrangement: &PeerArrangement,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(arrangement)?;
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Continuity",
                Some("org.axis.Shell.Continuity"),
                "SetPeerArrangement",
                &(&json,),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn update_peer_configs(
        &self,
        configs: &std::collections::HashMap<String, PeerConfig>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(configs)?;
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Continuity",
                Some("org.axis.Shell.Continuity"),
                "UpdatePeerConfigs",
                &(&json,),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    /// Convenience: update a single peer's config fields.
    /// Fetches current state, merges the update, and sends it.
    pub async fn update_peer_config_fields(
        &self,
        peer_id: &str,
        clipboard: Option<bool>,
        audio: Option<bool>,
        drag_drop: Option<bool>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let state = self.state();
        let mut configs = std::collections::HashMap::new();
        if let Some(current) = state.peer_configs.get(peer_id) {
            let mut updated = current.clone();
            if let Some(v) = clipboard { updated.clipboard = v; }
            if let Some(v) = audio { updated.audio = v; }
            if let Some(v) = drag_drop { updated.drag_drop = v; }
            updated.version += 1;
            configs.insert(peer_id.to_string(), updated);
        }
        if configs.is_empty() {
            return Ok(false);
        }
        self.update_peer_configs(&configs).await
    }
}

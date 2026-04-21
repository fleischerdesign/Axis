use std::cell::RefCell;
use std::rc::Rc;
use gtk4::glib;
use zbus::Connection;

use axis_core::services::network::dbus::NetworkStateSnapshot;

pub struct NetworkProxy {
    conn: Connection,
    cached: Rc<RefCell<NetworkStateSnapshot>>,
    listeners: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
}

impl NetworkProxy {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::session().await?;

        let _ = Self::call_get_state(&conn).await?;

        let proxy = Self {
            conn,
            cached: Rc::new(RefCell::new(NetworkStateSnapshot::default())),
            listeners: Rc::new(RefCell::new(Vec::new())),
        };

        proxy.start_signal_listener();

        if let Ok(state) = Self::call_get_state(&proxy.conn).await {
            *proxy.cached.borrow_mut() = state;
            Self::notify_listeners(&proxy.listeners);
        }

        Ok(proxy)
    }

    pub fn state(&self) -> NetworkStateSnapshot {
        self.cached.borrow().clone()
    }

    pub fn on_change(&self, f: impl Fn() + 'static) {
        f();
        self.listeners.borrow_mut().push(Box::new(f));
    }

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

    fn start_signal_listener(&self) {
        use futures_util::StreamExt;
        let conn = self.conn.clone();
        let cached = self.cached.clone();
        let listeners = self.listeners.clone();

        let _handle = glib::spawn_future_local(async move {
            let proxy = match zbus::Proxy::new(
                &conn,
                "org.axis.Shell",
                "/org/axis/Shell/Network",
                "org.axis.Shell.Network",
            )
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("[network-proxy] Failed to create signal proxy: {e}");
                    return;
                }
            };

            let mut stream = match proxy.receive_signal("StateChanged").await {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("[network-proxy] Failed to subscribe to StateChanged: {e}");
                    return;
                }
            };

            while let Some(signal) = stream.next().await {
                let body = signal.body();
                match body.deserialize::<(String,)>() {
                    Ok((json,)) => {
                        if let Ok(state) = serde_json::from_str::<NetworkStateSnapshot>(&json) {
                            *cached.borrow_mut() = state;
                            Self::notify_listeners(&listeners);
                        }
                    }
                    Err(e) => {
                        log::warn!("[network-proxy] Failed to parse signal: {e}");
                    }
                }
            }
        });
    }

    fn notify_listeners(listeners: &Rc<RefCell<Vec<Box<dyn Fn()>>>>) {
        let taken = std::mem::take(&mut *listeners.borrow_mut());
        for listener in &taken {
            listener();
        }
        *listeners.borrow_mut() = taken;
    }

    async fn call_get_state(
        conn: &Connection,
    ) -> Result<NetworkStateSnapshot, Box<dyn std::error::Error>> {
        let json: String = conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Network",
                Some("org.axis.Shell.Network"),
                "GetState",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(serde_json::from_str(&json)?)
    }

    pub async fn set_wifi_enabled(&self, enabled: bool) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Network",
                Some("org.axis.Shell.Network"),
                "SetWifiEnabled",
                &(enabled,),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn scan_wifi(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Network",
                Some("org.axis.Shell.Network"),
                "ScanWifi",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn connect_ap(&self, path: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Network",
                Some("org.axis.Shell.Network"),
                "ConnectAp",
                &(path,),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn connect_ap_with_password(&self, path: &str, ssid: &str, password: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Network",
                Some("org.axis.Shell.Network"),
                "ConnectApWithPassword",
                &(path, ssid, password),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn disconnect_wifi(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Network",
                Some("org.axis.Shell.Network"),
                "DisconnectWifi",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }
}

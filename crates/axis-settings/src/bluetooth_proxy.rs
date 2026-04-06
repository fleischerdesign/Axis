use std::cell::RefCell;
use std::rc::Rc;
use gtk4::glib;
use zbus::Connection;

use axis_core::services::bluetooth::dbus::BluetoothStateSnapshot;

pub struct BluetoothProxy {
    conn: Connection,
    cached: Rc<RefCell<BluetoothStateSnapshot>>,
    listeners: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
}

impl BluetoothProxy {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::session().await?;

        let _ = Self::call_get_state(&conn).await?;

        let proxy = Self {
            conn,
            cached: Rc::new(RefCell::new(BluetoothStateSnapshot::default())),
            listeners: Rc::new(RefCell::new(Vec::new())),
        };

        proxy.start_signal_listener();

        if let Ok(state) = Self::call_get_state(&proxy.conn).await {
            *proxy.cached.borrow_mut() = state;
            Self::notify_listeners(&proxy.listeners);
        }

        Ok(proxy)
    }

    pub fn state(&self) -> BluetoothStateSnapshot {
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
                "/org/axis/Shell/Bluetooth",
                "org.axis.Shell.Bluetooth",
            )
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("[bluetooth-proxy] Failed to create signal proxy: {e}");
                    return;
                }
            };

            let mut stream = match proxy.receive_signal("StateChanged").await {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("[bluetooth-proxy] Failed to subscribe to StateChanged: {e}");
                    return;
                }
            };

            while let Some(signal) = stream.next().await {
                let body = signal.body();
                match body.deserialize::<(String,)>() {
                    Ok((json,)) => {
                        if let Ok(state) = serde_json::from_str::<BluetoothStateSnapshot>(&json) {
                            *cached.borrow_mut() = state;
                            Self::notify_listeners(&listeners);
                        }
                    }
                    Err(e) => {
                        log::warn!("[bluetooth-proxy] Failed to parse signal: {e}");
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
    ) -> Result<BluetoothStateSnapshot, Box<dyn std::error::Error>> {
        let json: String = conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Bluetooth",
                Some("org.axis.Shell.Bluetooth"),
                "GetState",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(serde_json::from_str(&json)?)
    }

    pub async fn set_enabled(&self, enabled: bool) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Bluetooth",
                Some("org.axis.Shell.Bluetooth"),
                "SetEnabled",
                &(enabled,),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn connect_device(&self, path: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Bluetooth",
                Some("org.axis.Shell.Bluetooth"),
                "ConnectDevice",
                &(path,),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn disconnect_device(&self, path: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Bluetooth",
                Some("org.axis.Shell.Bluetooth"),
                "DisconnectDevice",
                &(path,),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn start_scan(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Bluetooth",
                Some("org.axis.Shell.Bluetooth"),
                "StartScan",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn stop_scan(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Bluetooth",
                Some("org.axis.Shell.Bluetooth"),
                "StopScan",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn accept_pairing(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Bluetooth",
                Some("org.axis.Shell.Bluetooth"),
                "AcceptPairing",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }

    pub async fn reject_pairing(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self.conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Bluetooth",
                Some("org.axis.Shell.Bluetooth"),
                "RejectPairing",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }
}

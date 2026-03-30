use std::cell::RefCell;
use std::rc::Rc;
use gtk4::glib;
use zbus::Connection;

use crate::config::*;

/// Typed D-Bus client for org.axis.Shell.Settings.
/// Subscribes to SettingsChanged signal and notifies listeners on config changes.
pub struct SettingsProxy {
    conn: Connection,
    cached: Rc<RefCell<AxisConfig>>,
    listeners: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
}

impl SettingsProxy {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::session().await?;
        let proxy = Self {
            conn,
            cached: Rc::new(RefCell::new(AxisConfig::default())),
            listeners: Rc::new(RefCell::new(Vec::new())),
        };
        proxy.reload_all().await?;
        proxy.start_signal_listener();
        Ok(proxy)
    }

    pub fn config(&self) -> AxisConfig {
        self.cached.borrow().clone()
    }

    /// Register a callback that fires when config changes (from D-Bus signal).
    pub fn on_change(&self, f: impl Fn() + 'static) {
        f(); // fire immediately with current state
        self.listeners.borrow_mut().push(Box::new(f));
    }

    // ── Signal Listener ─────────────────────────────────────────────────

    fn start_signal_listener(&self) {
        use futures_util::StreamExt;
        let conn = self.conn.clone();
        let cached = self.cached.clone();
        let listeners = self.listeners.clone();

        let _handle = glib::spawn_future_local(async move {
            // Create a proxy to receive signals
            let proxy = match zbus::Proxy::new(
                &conn,
                "org.axis.Shell",
                "/org/axis/Shell/Settings",
                "org.axis.Shell.Settings",
            )
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("[settings-proxy] Failed to create signal proxy: {e}");
                    return;
                }
            };

            let mut stream = match proxy.receive_signal("SettingsChanged").await {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("[settings-proxy] Failed to subscribe to SettingsChanged: {e}");
                    return;
                }
            };

            while let Some(signal) = stream.next().await {
                // SettingsChanged(section: &str, json: &str)
                let body = signal.body();
                match body.deserialize::<(String, String)>() {
                    Ok((_section, json)) => {
                        if let Ok(config) = serde_json::from_str::<AxisConfig>(&json) {
                            *cached.borrow_mut() = config;
                            let taken = std::mem::take(&mut *listeners.borrow_mut());
                            for listener in &taken {
                                listener();
                            }
                            *listeners.borrow_mut() = taken;
                        }
                    }
                    Err(e) => {
                        log::warn!("[settings-proxy] Failed to parse signal: {e}");
                    }
                }
            }
        });
    }

    // ── Load ────────────────────────────────────────────────────────────

    pub async fn reload_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json: String = self
            .conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Settings",
                Some("org.axis.Shell.Settings"),
                "GetAllSettings",
                &(),
            )
            .await?
            .body()
            .deserialize()?;
        let config: AxisConfig = serde_json::from_str(&json)?;
        *self.cached.borrow_mut() = config;
        Ok(())
    }

    // ── Setters ─────────────────────────────────────────────────────────

    pub async fn set_bar(&self, config: &BarConfig) -> Result<bool, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(config)?;
        self.call_set("SetBar", &json).await
    }

    pub async fn set_appearance(
        &self,
        config: &AppearanceConfig,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(config)?;
        self.call_set("SetAppearance", &json).await
    }

    pub async fn set_nightlight(
        &self,
        config: &NightlightConfig,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(config)?;
        self.call_set("SetNightlight", &json).await
    }

    pub async fn set_continuity(
        &self,
        config: &ContinuityConfig,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(config)?;
        self.call_set("SetContinuity", &json).await
    }

    pub async fn set_services(
        &self,
        config: &ServicesConfig,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(config)?;
        self.call_set("SetServices", &json).await
    }

    pub async fn set_shortcuts(
        &self,
        config: &ShortcutsConfig,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(config)?;
        self.call_set("SetShortcuts", &json).await
    }

    // ── Cache Updates (optimistic, before signal arrives) ───────────────

    pub fn update_cache_bar(&self, config: BarConfig) {
        self.cached.borrow_mut().bar = config;
    }
    pub fn update_cache_appearance(&self, config: AppearanceConfig) {
        self.cached.borrow_mut().appearance = config;
    }
    pub fn update_cache_nightlight(&self, config: NightlightConfig) {
        self.cached.borrow_mut().nightlight = config;
    }
    pub fn update_cache_continuity(&self, config: ContinuityConfig) {
        self.cached.borrow_mut().continuity = config;
    }
    pub fn update_cache_services(&self, config: ServicesConfig) {
        self.cached.borrow_mut().services = config;
    }
    pub fn update_cache_shortcuts(&self, config: ShortcutsConfig) {
        self.cached.borrow_mut().shortcuts = config;
    }

    // ── D-Bus Helpers ───────────────────────────────────────────────────

    async fn call_set(
        &self,
        method: &str,
        json: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let result: bool = self
            .conn
            .call_method(
                Some("org.axis.Shell"),
                "/org/axis/Shell/Settings",
                Some("org.axis.Shell.Settings"),
                method,
                &json,
            )
            .await?
            .body()
            .deserialize()?;
        Ok(result)
    }
}

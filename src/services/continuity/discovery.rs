use async_channel::Sender;
use futures_util::StreamExt;
use log::{error, info, warn};
use zbus::zvariant::OwnedObjectPath;
use zbus::Connection;

use super::PeerInfo;

// ── Discovery Events ───────────────────────────────────────────────────

#[derive(Debug)]
pub enum DiscoveryEvent {
    PeerFound(PeerInfo),
    PeerLost(String),
}

// ── Discovery Provider Trait ───────────────────────────────────────────

pub trait DiscoveryProvider: Send {
    fn register(&mut self, name: &str, port: u16) -> Result<(), String>;
    fn browse(&mut self, tx: Sender<DiscoveryEvent>) -> Result<(), String>;
    fn stop(&mut self);
}

// ── Avahi Discovery ────────────────────────────────────────────────────

const AVAHI_SERVICE: &str = "_axis-share._tcp";

pub struct AvahiDiscovery {
    browse_task: Option<tokio::task::JoinHandle<()>>,
}

impl AvahiDiscovery {
    pub fn new() -> Self {
        Self {
            browse_task: None,
        }
    }
}

impl DiscoveryProvider for AvahiDiscovery {
    fn register(&mut self, name: &str, port: u16) -> Result<(), String> {
        let name_owned = name.to_string();

        // Spawn registration as a non-blocking tokio task.
        tokio::spawn(async move {
            match register_service(&name_owned, port).await {
                Ok((_group_path, _conn)) => {
                    info!("[continuity:discovery] registration complete");
                }
                Err(e) => {
                    error!("[continuity:discovery] registration failed: {e}");
                }
            }
        });

        info!("[continuity:discovery] registration started for '{name}' on port {port}");
        Ok(())
    }

    fn browse(&mut self, tx: Sender<DiscoveryEvent>) -> Result<(), String> {
        // Browse creates its own D-Bus connection (independent of registration).
        let task = tokio::spawn(async move {
            let conn = match Connection::system().await {
                Ok(c) => c,
                Err(e) => {
                    error!("[continuity:discovery] browse connect failed: {e}");
                    return;
                }
            };
            if let Err(e) = browse_services(&conn, tx).await {
                error!("[continuity:discovery] browse error: {e}");
            }
        });

        self.browse_task = Some(task);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(task) = self.browse_task.take() {
            task.abort();
        }
    }
}

impl Drop for AvahiDiscovery {
    fn drop(&mut self) {
        self.stop();
    }
}

// ── Avahi D-Bus Helpers ────────────────────────────────────────────────

async fn register_service(
    name: &str,
    port: u16,
) -> Result<(OwnedObjectPath, Connection), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("D-Bus connect: {e}"))?;

    let server = zbus::Proxy::new(
        &conn,
        "org.freedesktop.Avahi",
        "/",
        "org.freedesktop.Avahi.Server",
    )
    .await
    .map_err(|e| format!("server proxy: {e}"))?;

    let group_path: OwnedObjectPath = server
        .call_method("EntryGroupNew", &())
        .await
        .map_err(|e| format!("EntryGroupNew: {e}"))?
        .body()
        .deserialize()
        .map_err(|e| format!("deserialize group path: {e}"))?;

    let group = zbus::Proxy::new(
        &conn,
        "org.freedesktop.Avahi",
        &group_path,
        "org.freedesktop.Avahi.EntryGroup",
    )
    .await
    .map_err(|e| format!("group proxy: {e}"))?;

    let empty: Vec<Vec<u8>> = Vec::new();

    group
        .call_method(
            "AddService",
            &(
                -1i32,    // interface: all
                -1i32,    // protocol: unspecified
                0u32,     // flags
                name,     // name
                AVAHI_SERVICE, // type
                "",       // domain (default)
                "",       // host (default)
                port,     // port
                empty,    // txt records (aay)
            ),
        )
        .await
        .map_err(|e| format!("AddService: {e}"))?;

    group
        .call_method("Commit", &())
        .await
        .map_err(|e| format!("Commit: {e}"))?;

    info!("[continuity:discovery] registered service '{name}' on port {port}");
    Ok((group_path, conn))
}

async fn browse_services(
    conn: &Connection,
    event_tx: Sender<DiscoveryEvent>,
) -> Result<(), String> {
    use futures_util::StreamExt;

    let server = zbus::Proxy::new(
        conn,
        "org.freedesktop.Avahi",
        "/",
        "org.freedesktop.Avahi.Server",
    )
    .await
    .map_err(|e| format!("server proxy: {e}"))?;

    let browser_path: OwnedObjectPath = server
        .call_method(
            "ServiceBrowserNew",
            &(-1i32, -1i32, AVAHI_SERVICE, "", 0u32),
        )
        .await
        .map_err(|e| format!("ServiceBrowserNew: {e}"))?
        .body()
        .deserialize()
        .map_err(|e| format!("deserialize browser path: {e}"))?;

    let browser = zbus::Proxy::new(
        conn,
        "org.freedesktop.Avahi",
        &browser_path,
        "org.freedesktop.Avahi.ServiceBrowser",
    )
    .await
    .map_err(|e| format!("browser proxy: {e}"))?;

    info!("[continuity:discovery] browsing for {AVAHI_SERVICE}");

    let mut item_new = browser
        .receive_signal("ItemNew")
        .await
        .map_err(|e| format!("ItemNew signal: {e}"))?;

    let mut item_remove = browser
        .receive_signal("ItemRemove")
        .await
        .map_err(|e| format!("ItemRemove signal: {e}"))?;

    loop {
        tokio::select! {
            Some(msg) = item_new.next() => {
                if let Ok((interface, protocol, name, stype, domain, flags)) =
                    msg.body().deserialize::<(i32, i32, String, String, String, u32)>()
                {
                    let conn_c = conn.clone();
                    let tx_c = event_tx.clone();
                    let name_c = name.clone();
                    tokio::spawn(async move {
                        match resolve_service(&conn_c, interface, protocol, &name_c, &stype, &domain).await {
                            Ok(peer) => {
                                info!("[continuity:discovery] found: {} at {}", peer.device_name, peer.address);
                                let _ = tx_c.send(DiscoveryEvent::PeerFound(peer)).await;
                            }
                            Err(e) => {
                                warn!("[continuity:discovery] resolve failed for {name_c}: {e}");
                            }
                        }
                    });
                }
            }
            Some(msg) = item_remove.next() => {
                if let Ok((_interface, _protocol, name, _stype, _domain, _flags)) =
                    msg.body().deserialize::<(i32, i32, String, String, String, u32)>()
                {
                    info!("[continuity:discovery] lost: {name}");
                    let _ = event_tx.send(DiscoveryEvent::PeerLost(name)).await;
                }
            }
            else => break,
        }
    }

    Ok(())
}

async fn resolve_service(
    conn: &Connection,
    interface: i32,
    protocol: i32,
    name: &str,
    stype: &str,
    domain: &str,
) -> Result<PeerInfo, String> {
    let server = zbus::Proxy::new(
        conn,
        "org.freedesktop.Avahi",
        "/",
        "org.freedesktop.Avahi.Server",
    )
    .await
    .map_err(|e| format!("server proxy: {e}"))?;

    let resolver_path: OwnedObjectPath = server
        .call_method(
            "ServiceResolverNew",
            &(interface, protocol, name, stype, domain, -1i32, 0u32),
        )
        .await
        .map_err(|e| format!("ServiceResolverNew: {e}"))?
        .body()
        .deserialize()
        .map_err(|e| format!("deserialize resolver path: {e}"))?;

    let resolver = zbus::Proxy::new(
        conn,
        "org.freedesktop.Avahi",
        &resolver_path,
        "org.freedesktop.Avahi.ServiceResolver",
    )
    .await
    .map_err(|e| format!("resolver proxy: {e}"))?;

    let mut found = resolver
        .receive_signal("Found")
        .await
        .map_err(|e| format!("Found signal: {e}"))?;

    if let Some(msg) = found.next().await {
        type AvahiFoundBody = (i32, i32, String, String, String, String, i32, String, u16, Vec<Vec<u8>>, u32);
        let body = msg.body();
        match body.deserialize::<AvahiFoundBody>() {
            Ok((_if, _proto, resolved_name, _stype, _domain, address, _a, _aa, port, _txt, _flags)) => {
                let device_id = resolved_name.clone();
                return Ok(PeerInfo {
                    device_id,
                    device_name: resolved_name,
                    address: format!("{address}:{port}")
                        .parse()
                        .map_err(|e| format!("parse address: {e}"))?,
                });
            }
            Err(e) => return Err(format!("deserialize Found: {e}")),
        }
    }

    Err("resolver timed out".into())
}

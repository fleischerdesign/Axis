use async_channel::Sender;
use futures_util::StreamExt;
use log::{error, info, warn};
use zbus::zvariant::OwnedObjectPath;
use zbus::Connection;

use super::{PeerInfo, known_peers};

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
    fn stop_browse(&mut self);
    fn stop(&mut self);
}

// ── Avahi Discovery ────────────────────────────────────────────────────

const AVAHI_SERVICE: &str = "_axis-share._tcp";

pub struct AvahiDiscovery {
    conn: Option<Connection>,
    entry_group_path: Option<OwnedObjectPath>,
    browse_task: Option<tokio::task::JoinHandle<()>>,
}

impl AvahiDiscovery {
    pub fn new() -> Self {
        Self {
            conn: None,
            entry_group_path: None,
            browse_task: None,
        }
    }
}

impl DiscoveryProvider for AvahiDiscovery {
    fn register(&mut self, name: &str, port: u16) -> Result<(), String> {
        let name = name.to_string();

        // Synchronous registration — we need to keep the connection and
        // EntryGroup alive. If we spawned a task and dropped the result,
        // Avahi would free the EntryGroup and unpublish the service.
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(register_service(&name, port));
            let _ = tx.send(result);
        });

        let (group_path, conn) = rx
            .recv()
            .map_err(|e| format!("register thread error: {e}"))??;

        self.conn = Some(conn);
        self.entry_group_path = Some(group_path);

        info!("[continuity:discovery] service registered and held alive");
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

    fn stop_browse(&mut self) {
        if let Some(task) = self.browse_task.take() {
            task.abort();
        }
    }

    fn stop(&mut self) {
        if let Some(task) = self.browse_task.take() {
            task.abort();
        }

        if let Some(conn) = &self.conn {
            if let Some(path) = self.entry_group_path.take() {
                let conn = conn.clone();
                tokio::spawn(async move {
                    if let Ok(group) = zbus::Proxy::new(
                        &conn,
                        "org.freedesktop.Avahi",
                        &path,
                        "org.freedesktop.Avahi.EntryGroup",
                    )
                    .await
                    {
                        let _ = group.call_method("Free", &()).await;
                    }
                });
            }
        }

        self.conn = None;
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

    // Avahi may cache services that registered before our browser started.
    // But if both peers started simultaneously, the cache might be empty.
    // Try resolving known services a few times to catch late registrations.
    let conn_scan = conn.clone();
    let tx_scan = event_tx.clone();
    tokio::spawn(async move {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for i in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            match scan_cached_services(&conn_scan).await {
                Ok(peers) => {
                    for peer in peers {
                        if seen.insert(peer.device_id.clone()) {
                            info!("[continuity:discovery] peer: {} at {}", peer.device_name, peer.address);
                            let _ = tx_scan.send(DiscoveryEvent::PeerFound(peer)).await;
                        }
                    }
                }
                Err(e) => {
                    if i == 0 {
                        warn!("[continuity:discovery] initial scan failed: {e}");
                    }
                }
            }
        }
    });

    loop {
        tokio::select! {
            Some(msg) = item_new.next() => {
                if let Ok((interface, protocol, name, stype, domain, _flags)) =
                    msg.body().deserialize::<(i32, i32, String, String, String, u32)>()
                {
                    // Only resolve IPv4 to avoid duplicate IPv4+IPv6 results
                    if protocol != 0 {
                        continue;
                    }

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
            Ok((_if, _proto, resolved_name, _stype, _domain, host, _a, address, port, _txt, _flags)) => {
                info!(
                    "[continuity:discovery] resolved: name={resolved_name} host={host} addr={address} port={port}"
                );

                let device_id = resolved_name.clone();

                // IPv6 needs brackets: [::1]:7391 vs IPv4: 192.168.1.1:7391
                let addr_str = if address.contains(':') {
                    format!("[{address}]:{port}")
                } else {
                    format!("{address}:{port}")
                };

                return Ok(PeerInfo {
                    device_id,
                    device_name: resolved_name,
                    hostname: host,
                    address: addr_str
                        .parse()
                        .map_err(|e| format!("parse address '{addr_str}': {e}"))?,
                    address_v6: None,
                });
            }
            Err(e) => return Err(format!("deserialize Found: {e}")),
        }
    }

    Err("resolver timed out".into())
}

/// Try to find Axis services via avahi-browse CLI.
/// Deduplicates by hostname, collects IPv4 + LAN-scope IPv6, skips self.
async fn scan_cached_services(
    _conn: &Connection,
) -> Result<Vec<PeerInfo>, String> {
    let output = tokio::process::Command::new("avahi-browse")
        .args(["-t", AVAHI_SERVICE, "--resolve", "-p", "-r"])
        .output()
        .await
        .map_err(|e| format!("avahi-browse: {e}"))?;

    let self_hostname = known_peers::hostname();

    // First pass: collect all valid addresses grouped by service name
    struct RawEntry {
        name: String,
        host: String,
        addr_v4: Option<std::net::SocketAddr>,
        addr_v6: Option<std::net::SocketAddr>,
    }
    let mut raw: std::collections::HashMap<String, RawEntry> = std::collections::HashMap::new();

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() < 9 || parts[0] != "=" {
            continue;
        }

        let name = parts[3].to_string();
        let host = parts[6].to_string();
        let address = parts[7].to_string();
        let port: u16 = parts[8].parse().unwrap_or(0);

        if port == 0 || address.is_empty() || name == self_hostname {
            continue;
        }
        if address == "127.0.0.1" || address == "::1" {
            continue;
        }
        if address.starts_with("172.17.") || address.starts_with("172.18.") {
            continue;
        }

        let is_ipv6 = address.contains(':');
        let addr_str = if is_ipv6 {
            format!("[{address}]:{port}")
        } else {
            format!("{address}:{port}")
        };
        let Ok(socket_addr) = addr_str.parse() else { continue };

        let entry = raw.entry(name.clone()).or_insert_with(|| RawEntry {
            name: name.clone(),
            host: host.clone(),
            addr_v4: None,
            addr_v6: None,
        });

        if is_ipv6 {
            let lower = address.to_lowercase();
            if lower.starts_with("fd") || lower.starts_with("fe80") {
                entry.addr_v6.get_or_insert(socket_addr);
            }
        } else {
            entry.addr_v4.get_or_insert(socket_addr);
        }
    }

    // Second pass: build PeerInfo (need IPv4 as primary)
    let mut result = Vec::new();
    for (_, entry) in raw {
        let Some(addr_v4) = entry.addr_v4 else { continue };
        result.push(PeerInfo {
            device_id: entry.name,
            device_name: entry.host.clone(),
            hostname: entry.host,
            address: addr_v4,
            address_v6: entry.addr_v6,
        });
    }

    Ok(result)
}

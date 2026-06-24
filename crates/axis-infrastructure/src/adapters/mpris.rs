use async_trait::async_trait;
use axis_domain::models::mpris::{MprisPlayer, MprisStatus, PlaybackState};
use axis_domain::ports::mpris::{MprisError, MprisProvider, MprisStream};
use futures_util::StreamExt;
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use zbus::zvariant::Value;
use zbus::{Connection, MatchRule, MessageStream, Proxy, proxy};

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_service = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MprisPlayerProxy {
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, Value<'static>>>;
    #[zbus(property)]
    fn position(&self) -> zbus::Result<i64>;
    #[zbus(property)]
    fn can_play(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn can_pause(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn can_go_next(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn can_go_previous(&self) -> zbus::Result<bool>;
    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;
}

pub struct MprisDBusProvider {
    status_tx: watch::Sender<MprisStatus>,
    position_tx: watch::Sender<(String, i64, i64)>,
    connection: Connection,
    name_owners: std::sync::Mutex<HashMap<String, String>>,
    position_polling: AtomicBool,
    pending_queries: std::sync::Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
}

impl MprisDBusProvider {
    pub async fn new() -> Result<Arc<Self>, MprisError> {
        let connection = Connection::session()
            .await
            .map_err(|e| MprisError::ProviderError(format!("D-Bus session connect: {e}")))?;

        let (status_tx, _) = watch::channel(MprisStatus::default());
        let (position_tx, _) = watch::channel((String::new(), 0, 0));

        let provider = Arc::new(Self {
            status_tx,
            position_tx,
            connection: connection.clone(),
            name_owners: std::sync::Mutex::new(HashMap::new()),
            position_polling: AtomicBool::new(false),
            pending_queries: std::sync::Mutex::new(HashMap::new()),
        });

        let initial_status = provider.discover_players().await;
        {
            let mut status = provider.status_tx.borrow().clone();
            status.players = initial_status;
            if !status.players.is_empty() {
                let active = status
                    .players
                    .iter()
                    .find(|p| p.playback != PlaybackState::Stopped);
                status.active_player_id = active.map(|p| p.id.clone());
                info!(
                    "[mpris] Setting initial status: {} players, active={:?}",
                    status.players.len(),
                    status.active_player_id
                );
            }
            provider.status_tx.send_replace(status);
        }

        let provider_clone = provider.clone();
        tokio::spawn(async move {
            provider_clone.listen_for_changes().await;
        });

        Ok(provider)
    }

    async fn discover_players(&self) -> Vec<MprisPlayer> {
        let dbus_proxy = match Proxy::new(
            &self.connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!("[mpris] Failed to create DBus proxy: {e}");
                return vec![];
            }
        };

        let reply = match dbus_proxy.call_method("ListNames", &()).await {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        let body = reply.body();
        let names: Vec<String> = match body.deserialize() {
            Ok(n) => n,
            Err(_) => return vec![],
        };

        let mut futures = Vec::new();
        for name in &names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                let name_clone = name.clone();
                futures.push(async move {
                    tokio::time::timeout(
                        std::time::Duration::from_millis(500),
                        self.query_player(&name_clone),
                    )
                    .await
                    .ok()
                    .flatten()
                });
            }
        }

        let queried_players = futures_util::future::join_all(futures).await;
        let mut players = Vec::new();
        for player in queried_players.into_iter().flatten() {
            players.push(player);
        }

        info!("[mpris] Discovered {} player(s)", players.len());
        for p in &players {
            info!(
                "[mpris]   player: id={}, title={:?}, artist={:?}, playback={:?}",
                p.id, p.title, p.artist, p.playback
            );
        }
        players
    }

    async fn query_player(&self, bus_name: &str) -> Option<MprisPlayer> {
        if let Ok(dbus_proxy) = Proxy::new(
            &self.connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await
            && let Ok(Ok(reply)) = tokio::time::timeout(
                std::time::Duration::from_millis(150),
                dbus_proxy.call_method("GetNameOwner", &(bus_name)),
            )
            .await
            && let Ok(owner) = reply.body().deserialize::<String>()
        {
            let mut owners = self.name_owners.lock().unwrap();
            owners.insert(owner, bus_name.to_string());
        }

        let proxy = match MprisPlayerProxyProxy::builder(&self.connection)
            .destination(bus_name)
            .ok()?
            .build()
            .await
        {
            Ok(p) => p,
            Err(_) => return None,
        };

        let playback_str = match tokio::time::timeout(
            std::time::Duration::from_millis(150),
            proxy.playback_status(),
        )
        .await
        {
            Ok(Ok(s)) => s,
            _ => return None,
        };
        let playback = match playback_str.to_lowercase().as_str() {
            "playing" => PlaybackState::Playing,
            "paused" => PlaybackState::Paused,
            _ => PlaybackState::Stopped,
        };

        let metadata =
            match tokio::time::timeout(std::time::Duration::from_millis(150), proxy.metadata())
                .await
            {
                Ok(Ok(m)) => m,
                _ => HashMap::new(),
            };
        let (title, artist, album, art_url, length_us) = extract_metadata(&metadata);

        let position_us =
            match tokio::time::timeout(std::time::Duration::from_millis(150), proxy.position())
                .await
            {
                Ok(Ok(p)) => p,
                _ => 0,
            };

        let id = bus_name
            .trim_start_matches("org.mpris.MediaPlayer2.")
            .to_string();

        let can_play = matches!(
            tokio::time::timeout(std::time::Duration::from_millis(150), proxy.can_play()).await,
            Ok(Ok(true))
        );
        let can_pause = matches!(
            tokio::time::timeout(std::time::Duration::from_millis(150), proxy.can_pause()).await,
            Ok(Ok(true))
        );
        let can_go_next = matches!(
            tokio::time::timeout(std::time::Duration::from_millis(150), proxy.can_go_next()).await,
            Ok(Ok(true))
        );
        let can_go_previous = matches!(
            tokio::time::timeout(
                std::time::Duration::from_millis(150),
                proxy.can_go_previous()
            )
            .await,
            Ok(Ok(true))
        );

        Some(MprisPlayer {
            id,
            title,
            artist,
            album,
            art_url,
            playback,
            position_us,
            length_us,
            can_play,
            can_pause,
            can_go_next,
            can_go_previous,
        })
    }

    fn resolve_bus_name(&self, name: &str) -> String {
        if name.starts_with("org.mpris.MediaPlayer2.") {
            return name.to_string();
        }
        self.name_owners
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    fn trigger_query(self: &Arc<Self>, bus_name: String) {
        let provider = self.clone();
        let mut pending = self.pending_queries.lock().unwrap();
        if let Some(handle) = pending.get(&bus_name) {
            handle.abort();
        }
        let bus_name_clone = bus_name.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            if let Some(player) = tokio::time::timeout(
                std::time::Duration::from_millis(500),
                provider.query_player(&bus_name_clone),
            )
            .await
            .ok()
            .flatten()
            {
                provider.update_player(player);
            }
            provider
                .pending_queries
                .lock()
                .unwrap()
                .remove(&bus_name_clone);
        });
        pending.insert(bus_name, handle);
    }

    pub fn set_position_polling(&self, enabled: bool) {
        self.position_polling.store(enabled, Ordering::Relaxed);
    }

    pub fn subscribe_positions(&self) -> watch::Receiver<(String, i64, i64)> {
        self.position_tx.subscribe()
    }

    async fn listen_for_changes(self: Arc<Self>) {
        let name_rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.freedesktop.DBus")
            .unwrap()
            .member("NameOwnerChanged")
            .unwrap()
            .arg0ns("org.mpris.MediaPlayer2")
            .unwrap()
            .build();

        let mut name_stream =
            match MessageStream::for_match_rule(name_rule, &self.connection, None).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("[mpris] Failed to subscribe to NameOwnerChanged: {e}");
                    return;
                }
            };

        let props_rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .unwrap()
            .member("PropertiesChanged")
            .unwrap()
            .path_namespace("/org/mpris/MediaPlayer2")
            .unwrap()
            .build();

        let mut props_stream =
            match MessageStream::for_match_rule(props_rule, &self.connection, None).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("[mpris] Failed to subscribe to PropertiesChanged: {e}");
                    return;
                }
            };

        info!("[mpris] Listening for changes");

        let mut pos_tick = tokio::time::interval(std::time::Duration::from_millis(250));
        pos_tick.tick().await;
        let mut requery_tick = tokio::time::interval(std::time::Duration::from_secs(5));
        requery_tick.tick().await;

        loop {
            tokio::select! {
                Some(msg) = name_stream.next() => {
                    let msg = match msg {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let body = msg.body();
                    let result: Result<(&str, &str, &str), _> = body.deserialize();
                    let Ok((name, old, new)) = result else { continue; };

                    if new.is_empty() {
                        info!("[mpris] Player disappeared: {name}");
                        {
                            let mut owners = self.name_owners.lock().unwrap();
                            owners.remove(old);
                        }
                        self.remove_player(name);
                    } else {
                        info!("[mpris] Player appeared: {name}");
                        {
                            let mut owners = self.name_owners.lock().unwrap();
                            if !old.is_empty() {
                                owners.remove(old);
                            }
                            owners.insert(new.to_string(), name.to_string());
                        }
                        self.trigger_query(name.to_string());
                    }
                }
                Some(msg) = props_stream.next() => {
                    let msg = match msg {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let header = msg.header();
                    let Some(sender) = header.sender() else { continue };
                    let sender_str = sender.as_str().to_string();

                    let bus_name = self.resolve_bus_name(&sender_str);
                    if bus_name.starts_with("org.mpris.MediaPlayer2.") {
                        self.trigger_query(bus_name);
                    }
                }
                _ = pos_tick.tick() => {
                    if !self.position_polling.load(Ordering::Relaxed) {
                        continue;
                    }
                    let (bus_name, id, length_us) = {
                        let s = self.status_tx.borrow();
                        match s.active_player() {
                            Some(p) if p.playback == PlaybackState::Playing => {
                                (format!("org.mpris.MediaPlayer2.{}", p.id), p.id.clone(), p.length_us)
                            }
                            _ => continue,
                        }
                    };
                    let Ok(proxy) = MprisPlayerProxyProxy::builder(&self.connection)
                        .destination(bus_name.as_str())
                        .unwrap()
                        .build()
                        .await
                    else { continue };
                    let Ok(Ok(pos)) = tokio::time::timeout(
                        std::time::Duration::from_millis(150),
                        proxy.position()
                    ).await else { continue };
                    self.position_tx.send_replace((id, pos, length_us));
                }
                _ = requery_tick.tick() => {
                    let stopped: Vec<String> = {
                        let s = self.status_tx.borrow();
                        s.players
                            .iter()
                            .filter(|p| p.playback == PlaybackState::Stopped)
                            .map(|p| format!("org.mpris.MediaPlayer2.{}", p.id))
                            .collect()
                    };
                    for bus_name in stopped {
                        self.trigger_query(bus_name);
                    }
                }
            }
        }
    }

    fn update_player(&self, player: MprisPlayer) {
        let mut status = self.status_tx.borrow().clone();
        if let Some(existing) = status.players.iter_mut().find(|p| p.id == player.id) {
            *existing = player;
        } else {
            status.players.push(player);
        }
        status.active_player_id =
            self.determine_active_player(&status.players, status.active_player_id.as_deref());
        self.status_tx.send_replace(status);
    }

    fn remove_player(&self, bus_name: &str) {
        let id = bus_name.trim_start_matches("org.mpris.MediaPlayer2.");
        let mut status = self.status_tx.borrow().clone();
        status.players.retain(|p| p.id != id);
        status.active_player_id =
            self.determine_active_player(&status.players, status.active_player_id.as_deref());
        self.status_tx.send_replace(status);
    }

    fn determine_active_player(
        &self,
        players: &[MprisPlayer],
        current_active: Option<&str>,
    ) -> Option<String> {
        if players.is_empty() {
            return None;
        }
        let playing: Vec<&MprisPlayer> = players
            .iter()
            .filter(|p| p.playback == PlaybackState::Playing)
            .collect();
        if !playing.is_empty() {
            if let Some(active) = current_active
                && playing.iter().any(|p| p.id == active)
            {
                return Some(active.to_string());
            }
            return Some(playing[0].id.clone());
        }
        let paused: Vec<&MprisPlayer> = players
            .iter()
            .filter(|p| p.playback == PlaybackState::Paused)
            .collect();
        if !paused.is_empty() {
            if let Some(active) = current_active
                && paused.iter().any(|p| p.id == active)
            {
                return Some(active.to_string());
            }
            return Some(paused[0].id.clone());
        }
        let stopped: Vec<&MprisPlayer> = players
            .iter()
            .filter(|p| p.playback == PlaybackState::Stopped)
            .collect();
        if !stopped.is_empty() {
            if let Some(active) = current_active
                && stopped.iter().any(|p| p.id == active)
            {
                return Some(active.to_string());
            }
            return Some(stopped[0].id.clone());
        }
        None
    }
}

fn extract_metadata(
    metadata: &HashMap<String, Value<'static>>,
) -> (String, String, String, Option<String>, i64) {
    let title = metadata
        .get("xesam:title")
        .and_then(|v| {
            if let Value::Str(s) = v {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("")
        .to_string();

    let artist = metadata
        .get("xesam:artist")
        .and_then(|v| {
            if let Value::Array(arr) = v {
                let artists: Vec<&str> = arr
                    .iter()
                    .filter_map(|val| {
                        if let Value::Str(s) = val {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                if artists.is_empty() {
                    None
                } else {
                    Some(artists.join(", "))
                }
            } else {
                None
            }
        })
        .unwrap_or_default();

    let album = metadata
        .get("xesam:album")
        .and_then(|v| {
            if let Value::Str(s) = v {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("")
        .to_string();

    let art_url = metadata.get("mpris:artUrl").and_then(|v| {
        if let Value::Str(s) = v {
            Some(s.as_str().to_string())
        } else {
            None
        }
    });

    let length_us = metadata
        .get("mpris:length")
        .and_then(|v| {
            if let Value::I64(n) = v {
                Some(*n)
            } else if let Value::U64(n) = v {
                Some(*n as i64)
            } else {
                None
            }
        })
        .unwrap_or(0);

    (title, artist, album, art_url, length_us)
}

#[async_trait]
impl MprisProvider for MprisDBusProvider {
    async fn get_status(&self) -> Result<MprisStatus, MprisError> {
        let s = self.status_tx.borrow().clone();
        info!(
            "[mpris] get_status: {} players, active={:?}",
            s.players.len(),
            s.active_player_id
        );
        Ok(s)
    }

    async fn subscribe(&self) -> Result<MprisStream, MprisError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn play_pause(&self, player_id: &str) -> Result<(), MprisError> {
        let bus_name = format!("org.mpris.MediaPlayer2.{player_id}");
        let proxy = MprisPlayerProxyProxy::builder(&self.connection)
            .destination(bus_name.as_str())
            .map_err(|e| MprisError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| MprisError::ProviderError(format!("proxy build: {e}")))?;
        proxy
            .play_pause()
            .await
            .map_err(|e| MprisError::ProviderError(e.to_string()))
    }

    async fn next(&self, player_id: &str) -> Result<(), MprisError> {
        let bus_name = format!("org.mpris.MediaPlayer2.{player_id}");
        let proxy = MprisPlayerProxyProxy::builder(&self.connection)
            .destination(bus_name.as_str())
            .map_err(|e| MprisError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| MprisError::ProviderError(format!("proxy build: {e}")))?;
        proxy
            .next()
            .await
            .map_err(|e| MprisError::ProviderError(e.to_string()))
    }

    async fn previous(&self, player_id: &str) -> Result<(), MprisError> {
        let bus_name = format!("org.mpris.MediaPlayer2.{player_id}");
        let proxy = MprisPlayerProxyProxy::builder(&self.connection)
            .destination(bus_name.as_str())
            .map_err(|e| MprisError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| MprisError::ProviderError(format!("proxy build: {e}")))?;
        proxy
            .previous()
            .await
            .map_err(|e| MprisError::ProviderError(e.to_string()))
    }
}

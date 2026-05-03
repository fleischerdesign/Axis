use axis_domain::models::mpris::{MprisPlayer, MprisStatus, PlaybackState};
use axis_domain::ports::mpris::{MprisError, MprisProvider, MprisStream};
use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use zbus::{proxy, Connection, Proxy, MessageStream, MatchRule};
use zbus::zvariant::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use log::{info, warn};

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
        });

        let initial_status = provider.discover_players().await;
        {
            let mut status = provider.status_tx.borrow().clone();
            status.players = initial_status;
            if !status.players.is_empty() {
            let playing = status.players.iter().find(|p| p.playback == PlaybackState::Playing);
            status.active_player_id = playing
                .or(status.players.first())
                .map(|p| p.id.clone());
            info!("[mpris] Setting initial status: {} players, active={:?}", status.players.len(), status.active_player_id);
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

        let mut players = Vec::new();
        let mut owners = self.name_owners.lock().unwrap();
        for name in &names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                if let Some(player) = self.query_player(name).await {
                    if let Ok(reply) = dbus_proxy.call_method("GetNameOwner", &(name.as_str())).await {
                        if let Ok(owner) = reply.body().deserialize::<String>() {
                            owners.insert(owner, name.clone());
                        }
                    }
                    players.push(player);
                }
            }
        }
        drop(owners);

        info!("[mpris] Discovered {} player(s)", players.len());
        for p in &players {
            info!("[mpris]   player: id={}, title={:?}, artist={:?}, playback={:?}", p.id, p.title, p.artist, p.playback);
        }
        players
    }

    async fn query_player(&self, bus_name: &str) -> Option<MprisPlayer> {
        let proxy = match MprisPlayerProxyProxy::builder(&self.connection)
            .destination(bus_name)
            .ok()?
            .build()
            .await
        {
            Ok(p) => p,
            Err(_) => return None,
        };

        let playback_str = proxy.playback_status().await.ok()?;
        let playback = match playback_str.as_str() {
            "Playing" => PlaybackState::Playing,
            "Paused" => PlaybackState::Paused,
            _ => PlaybackState::Stopped,
        };

        let metadata = proxy.metadata().await.ok().unwrap_or_default();
        let (title, artist, album, art_url, length_us) = extract_metadata(&metadata);

        let position_us = proxy.position().await.unwrap_or(0);

        let id = bus_name.trim_start_matches("org.mpris.MediaPlayer2.").to_string();

        Some(MprisPlayer {
            id,
            title,
            artist,
            album,
            art_url,
            playback,
            position_us,
            length_us,
            can_play: proxy.can_play().await.unwrap_or(false),
            can_pause: proxy.can_pause().await.unwrap_or(false),
            can_go_next: proxy.can_go_next().await.unwrap_or(false),
            can_go_previous: proxy.can_go_previous().await.unwrap_or(false),
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

    pub fn set_position_polling(&self, enabled: bool) {
        self.position_polling.store(enabled, Ordering::Relaxed);
    }

    pub fn subscribe_positions(&self) -> watch::Receiver<(String, i64, i64)> {
        self.position_tx.subscribe()
    }

    async fn listen_for_changes(&self) {
        let name_rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.freedesktop.DBus")
            .unwrap()
            .member("NameOwnerChanged")
            .unwrap()
            .arg0ns("org.mpris.MediaPlayer2")
            .unwrap()
            .build();

        let mut name_stream = match MessageStream::for_match_rule(name_rule, &self.connection, None).await {
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

        let mut props_stream = match MessageStream::for_match_rule(props_rule, &self.connection, None).await
        {
            Ok(s) => s,
            Err(e) => {
                warn!("[mpris] Failed to subscribe to PropertiesChanged: {e}");
                return;
            }
        };

        info!("[mpris] Listening for changes");

        let mut last_query: HashMap<String, Instant> = HashMap::new();
        let mut pos_tick = tokio::time::interval(std::time::Duration::from_millis(250));
        pos_tick.tick().await;

        loop {
            tokio::select! {
                Some(msg) = name_stream.next() => {
                    let msg = match msg {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let body = msg.body();
                    let result: Result<(&str, &str, &str), _> = body.deserialize();
                    let Ok((name, _old, new)) = result else { continue; };

                    if new.is_empty() {
                        info!("[mpris] Player disappeared: {name}");
                        last_query.remove(name);
                        self.remove_player(name);
                    } else {
                        info!("[mpris] Player appeared: {name}");
                        {
                            let mut owners = self.name_owners.lock().unwrap();
                            owners.insert(new.to_string(), name.to_string());
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        if let Some(player) = tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            self.query_player(name)
                        ).await.ok().flatten() {
                            self.update_player(player);
                        }
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
                    if !bus_name.starts_with("org.mpris.MediaPlayer2.") {
                        continue;
                    }

                    let now = Instant::now();
                    if let Some(last) = last_query.get(&bus_name) {
                        if now.duration_since(*last) < std::time::Duration::from_millis(200) {
                            continue;
                        }
                    }
                    last_query.insert(bus_name.clone(), now);

                    if let Some(player) = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        self.query_player(&bus_name)
                    ).await.ok().flatten() {
                        self.update_player(player);
                    } else {
                        warn!("[mpris] query_player timed out for {bus_name}");
                    }
                }
                _ = pos_tick.tick() => {
                    if !self.position_polling.load(Ordering::Relaxed) {
                        continue;
                    }
                    let (bus_name, id, length_us) = {
                        let s = self.status_tx.borrow();
                        match s.active_player() {
                            Some(p) if p.playback == PlaybackState::Playing && p.length_us > 0 => {
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
                    let Ok(pos) = proxy.position().await else { continue };
                    self.position_tx.send_replace((id, pos, length_us));
                }
            }
        }
    }

    fn update_player(&self, player: MprisPlayer) {
        let mut status = self.status_tx.borrow().clone();
        if let Some(existing) = status.players.iter_mut().find(|p| p.id == player.id) {
            let was_playing = existing.playback == PlaybackState::Playing;
            *existing = player.clone();
            if player.playback == PlaybackState::Playing && !was_playing {
                info!("[mpris] Now playing: {} — {}", player.artist, player.title);
                status.active_player_id = Some(player.id.clone());
            }
        } else {
            if player.playback == PlaybackState::Playing || status.active_player_id.is_none() {
                info!("[mpris] New active player: {} — {}", player.id, player.title);
                status.active_player_id = Some(player.id.clone());
            }
            status.players.push(player);
        }
        self.status_tx.send_replace(status);
    }

    fn remove_player(&self, bus_name: &str) {
        let id = bus_name.trim_start_matches("org.mpris.MediaPlayer2.");
        let mut status = self.status_tx.borrow().clone();
        status.players.retain(|p| p.id != id);
        if status.active_player_id.as_deref() == Some(id) {
            status.active_player_id = status.players.first().map(|p| p.id.clone());
        }
        self.status_tx.send_replace(status);
    }
}

fn extract_metadata(metadata: &HashMap<String, Value<'static>>) -> (String, String, String, Option<String>, i64) {
    let title = metadata
        .get("xesam:title")
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .unwrap_or("")
        .to_string();

    let artist = metadata
        .get("xesam:artist")
        .and_then(|v| {
            if let Value::Array(arr) = v {
                arr.iter().next().and_then(|v| {
                    if let Value::Str(s) = v { Some(s.as_str()) } else { None }
                })
            } else {
                None
            }
        })
        .unwrap_or("")
        .to_string();

    let album = metadata
        .get("xesam:album")
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .unwrap_or("")
        .to_string();

    let art_url = metadata
        .get("mpris:artUrl")
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str().to_string()) } else { None });

    let length_us = metadata
        .get("mpris:length")
        .and_then(|v| {
            if let Value::I64(n) = v { Some(*n) }
            else if let Value::U64(n) = v { Some(*n as i64) }
            else { None }
        })
        .unwrap_or(0);

    (title, artist, album, art_url, length_us)
}

#[async_trait]
impl MprisProvider for MprisDBusProvider {
    async fn get_status(&self) -> Result<MprisStatus, MprisError> {
        let s = self.status_tx.borrow().clone();
        info!("[mpris] get_status: {} players, active={:?}", s.players.len(), s.active_player_id);
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
        proxy.play_pause().await.map_err(|e| MprisError::ProviderError(e.to_string()))
    }

    async fn next(&self, player_id: &str) -> Result<(), MprisError> {
        let bus_name = format!("org.mpris.MediaPlayer2.{player_id}");
        let proxy = MprisPlayerProxyProxy::builder(&self.connection)
            .destination(bus_name.as_str())
            .map_err(|e| MprisError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| MprisError::ProviderError(format!("proxy build: {e}")))?;
        proxy.next().await.map_err(|e| MprisError::ProviderError(e.to_string()))
    }

    async fn previous(&self, player_id: &str) -> Result<(), MprisError> {
        let bus_name = format!("org.mpris.MediaPlayer2.{player_id}");
        let proxy = MprisPlayerProxyProxy::builder(&self.connection)
            .destination(bus_name.as_str())
            .map_err(|e| MprisError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| MprisError::ProviderError(format!("proxy build: {e}")))?;
        proxy.previous().await.map_err(|e| MprisError::ProviderError(e.to_string()))
    }
}

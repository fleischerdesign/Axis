use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Side {
    #[default]
    Right,
    Left,
    Top,
    Bottom,
}

impl Side {
    pub fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum SharingState {
    #[default]
    Idle,
    Pending {
        entry_side: Side,
        edge_pos: f64,
    },
    Sharing {
        entry_side: Side,
        virtual_pos: (f64, f64),
    },
    Receiving,
    PendingSwitch,
}

impl SharingState {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn is_active(&self) -> bool {
        !self.is_idle()
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Pending { .. } => "Pending",
            Self::Sharing { .. } => "Sharing",
            Self::Receiving => "Receiving",
            Self::PendingSwitch => "PendingSwitch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub device_id: String,
    pub device_name: String,
    pub hostname: String,
    pub address: SocketAddr,
    pub address_v6: Option<SocketAddr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveConnectionInfo {
    pub peer_id: String,
    pub peer_name: String,
    pub connected_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPin {
    pub pin: String,
    pub peer_id: String,
    pub peer_name: String,
    pub is_incoming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerArrangement {
    pub side: Side,
    pub offset: i32,
}

impl PeerArrangement {
    pub fn overlap_on_local(&self, local_len: i32, remote_len: i32) -> Option<(i32, i32)> {
        let start = self.offset.max(0);
        let end = (self.offset + remote_len).min(local_len);
        if start < end {
            Some((start, end))
        } else {
            None
        }
    }

    pub fn overlap_on_remote(&self, local_len: i32, remote_len: i32) -> Option<(i32, i32)> {
        let start = (-self.offset).max(0);
        let end = (local_len - self.offset).min(remote_len);
        if start < end {
            Some((start, end))
        } else {
            None
        }
    }

    pub fn local_to_remote_edge(&self, local_pos: f64) -> f64 {
        local_pos - self.offset as f64
    }

    pub fn remote_to_local_edge(&self, remote_pos: f64) -> f64 {
        remote_pos + self.offset as f64
    }

    pub fn local_edge_length(&self, screen_w: i32, screen_h: i32) -> i32 {
        match self.side {
            Side::Left | Side::Right => screen_h,
            Side::Top | Side::Bottom => screen_w,
        }
    }
}

impl Default for PeerArrangement {
    fn default() -> Self {
        Self {
            side: Side::Right,
            offset: 0,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerConfig {
    pub trusted: bool,
    #[serde(default = "default_true")]
    pub auto_connect: bool,
    pub arrangement: PeerArrangement,
    pub clipboard: bool,
    pub audio: bool,
    pub drag_drop: bool,
    pub version: u64,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            trusted: false,
            auto_connect: true,
            arrangement: PeerArrangement::default(),
            clipboard: true,
            audio: false,
            drag_drop: false,
            version: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputGeometry {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconnectState {
    pub peer_id: String,
    pub peer_name: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveDragPayload {
    pub transfer_id: String,
    pub name: String,
    pub size_bytes: u64,
    pub mime_type: String,
    pub is_directory: bool,
    pub item_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuityStatus {
    pub device_id: String,
    pub device_name: String,
    pub enabled: bool,
    pub peers: Vec<PeerInfo>,
    pub active_connection: Option<ActiveConnectionInfo>,
    pub sharing_state: SharingState,
    pub pending_pin: Option<PendingPin>,
    pub peer_configs: HashMap<String, PeerConfig>,
    pub screen_width: i32,
    pub screen_height: i32,
    pub local_outputs: Vec<OutputGeometry>,
    pub remote_screen: Option<(i32, i32)>,
    pub reconnect: Option<ReconnectState>,
    pub active_drag: Option<ActiveDragPayload>,
    pub connecting_peer_id: Option<String>,
}

impl ContinuityStatus {
    pub fn active_peer_config(&self) -> PeerConfig {
        if let Some(conn) = &self.active_connection {
            self.peer_configs
                .get(&conn.peer_id)
                .or_else(|| {
                    self.peers
                        .iter()
                        .find(|p| {
                            p.device_name == conn.peer_name
                                || p.hostname == conn.peer_name
                                || p.device_id == conn.peer_id
                        })
                        .and_then(|p| self.peer_configs.get(&p.device_id))
                })
                .or_else(|| self.peer_configs.values().next())
                .cloned()
                .unwrap_or_default()
        } else {
            PeerConfig::default()
        }
    }
}

impl Default for ContinuityStatus {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            device_name: String::new(),
            enabled: false,
            peers: Vec::new(),
            active_connection: None,
            sharing_state: SharingState::default(),
            pending_pin: None,
            peer_configs: HashMap::new(),
            screen_width: 1920,
            screen_height: 1080,
            local_outputs: Vec::new(),
            remote_screen: None,
            reconnect: None,
            active_drag: None,
            connecting_peer_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Handshake {
        device_id: String,
        device_name: String,
        version: u32,
    },
    PinRequest {
        pin: String,
    },
    PinConfirm {
        pin: String,
    },
    ScreenInfo {
        width: i32,
        height: i32,
    },
    ConfigSync {
        arrangement: Side,
        offset: i32,
        clipboard: bool,
        audio: bool,
        drag_drop: bool,
        version: u64,
    },
    ClipboardUpdate {
        content: Vec<u8>,
        mime_type: String,
    },
    DragOffer {
        transfer_id: String,
        file_name: String,
        file_size: u64,
        mime_type: String,
        is_directory: bool,
        item_count: u32,
    },
    DragChunk {
        transfer_id: String,
        chunk_index: u32,
        is_last: bool,
        data: Vec<u8>,
    },
    DragCancel {
        transfer_id: String,
    },
    NotificationOffer {
        notification_id: String,
        app_name: String,
        title: String,
        body: String,
        icon: String,
    },
    NotificationDismissed {
        notification_id: String,
    },
    NotificationActionInvoked {
        notification_id: String,
        action_key: String,
    },
    AudioChunk {
        channel_id: u8,
        pcm_data: Vec<u8>,
    },
    EdgeTransition {
        side: Side,
        edge_pos: f64,
    },
    TransitionAck {
        accepted: bool,
    },
    TransitionCancel,
    SwitchTransition {
        side: Side,
        edge_pos: f64,
    },
    SwitchConfirm {
        side: Side,
        edge_pos: f64,
    },
    CursorMove {
        dx: f64,
        dy: f64,
    },
    KeyPress {
        key: u32,
        state: u32,
    },
    KeyRelease {
        key: u32,
    },
    PointerButton {
        button: u32,
        state: u32,
    },
    PointerAxis {
        dx: f64,
        dy: f64,
    },
    Connected,
    Heartbeat,
    Disconnect {
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    CursorMove { dx: f64, dy: f64 },
    KeyPress { key: u32, state: u32 },
    KeyRelease { key: u32 },
    PointerButton { button: u32, state: u32 },
    PointerAxis { dx: f64, dy: f64 },
    EmergencyExit,
}

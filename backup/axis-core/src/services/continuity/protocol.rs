use serde::{Deserialize, Serialize};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAGIC: &[u8; 4] = b"AXIS";
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

// ── Message Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Handshake
    Hello {
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
    Connected,

    // Screen info (exchanged after handshake)
    ScreenInfo {
        width: i32,
        height: i32,
    },

    // Configuration sync (arrangement, settings, versioning)
    ConfigSync {
        arrangement: Side,
        offset: i32,
        clipboard: bool,
        audio: bool,
        drag_drop: bool,
        version: u64,
    },

    // Cursor Transition
    EdgeTransition {
        side: Side,
        /// Position along the shared edge in the remote peer's screen coordinates.
        /// For Left/Right edges this is a Y coordinate, for Top/Bottom it's X.
        edge_pos: f64,
    },
    TransitionAck {
        accepted: bool,
    },
    TransitionCancel,
    SwitchTransition {
        side: Side,
        /// Position along the shared edge in the sender's local coordinates.
        edge_pos: f64,
    },
    SwitchConfirm {
        side: Side,
        /// The old sharer's virtual_pos along the edge (in remote screen coords,
        /// i.e. the SwitchConfirm-sender's own screen coordinates).
        edge_pos: f64,
    },

    // Input (forwarded when Driving)
    CursorMove {
        dx: f64,
        dy: f64,
    },
    KeyPress {
        key: u32,
        state: u8,
    },
    KeyRelease {
        key: u32,
    },
    PointerButton {
        button: u32,
        state: u8,
    },
    PointerAxis {
        dx: f64,
        dy: f64,
    },

    // Clipboard
    ClipboardUpdate {
        content: Vec<u8>,
        mime_type: String,
    },

    // Control
    Heartbeat,
    Disconnect {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
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

// ── Framing: length-prefixed JSON ──────────────────────────────────────
// Wire format: [4 bytes magic][4 bytes version][4 bytes length][JSON payload]

pub async fn write_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &Message,
) -> io::Result<()> {
    let payload = serde_json::to_vec(msg)?;
    let len = payload.len() as u32;

    let mut wire = Vec::with_capacity(12 + payload.len());
    wire.extend_from_slice(MAGIC);
    wire.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    wire.extend_from_slice(&len.to_be_bytes());
    wire.extend_from_slice(&payload);

    log::trace!(
        "[continuity:protocol] TX {} bytes: {:02x?}",
        wire.len(),
        &wire[..wire.len().min(24)]
    );

    writer.write_all(&wire).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_message<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> io::Result<Message> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).await?;

    log::debug!(
        "[continuity:protocol] RX magic: {:02x?} (expect {:02x?})",
        magic,
        MAGIC
    );

    if &magic != MAGIC {
        // Read remaining header bytes for debugging
        let mut ver = [0u8; 4];
        let mut len_bytes = [0u8; 4];
        let _ = reader.read_exact(&mut ver).await;
        let _ = reader.read_exact(&mut len_bytes).await;

        log::debug!(
            "[continuity:protocol] RX BAD: magic={magic:02x?} ver={ver:02x?} len={len_bytes:02x?}"
        );
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid magic bytes",
        ));
    }

    // Read version and length
    let mut ver_bytes = [0u8; 4];
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut ver_bytes).await?;
    reader.read_exact(&mut len_bytes).await?;

    let version = u32::from_be_bytes(ver_bytes);
    let len = u32::from_be_bytes(len_bytes) as usize;

    log::debug!(
        "[continuity:protocol] RX header: ver={version} len={len}"
    );

    if version != PROTOCOL_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported protocol version: {version}"),
        ));
    }

    if len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "message too large",
        ));
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;

    serde_json::from_slice(&payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

use serde::{Deserialize, Serialize};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAGIC: &[u8; 4] = b"AXIS";

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

    // Cursor Transition
    EdgeTransition {
        side: Side,
    },
    TransitionAck {
        accepted: bool,
    },
    TransitionCancel,

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
        content: String,
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

    log::debug!("[continuity:protocol] writing {} bytes (magic + {} payload)", len as usize + 12, len);
    writer.write_all(MAGIC).await?;
    writer.write_all(&PROTOCOL_VERSION.to_be_bytes()).await?;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_message<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> io::Result<Message> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).await?;
    if &magic != MAGIC {
        log::warn!(
            "[continuity:protocol] invalid magic bytes: {:02x?} (expected {:02x?})",
            magic,
            MAGIC
        );
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid magic bytes",
        ));
    }

    let mut version_bytes = [0u8; 4];
    reader.read_exact(&mut version_bytes).await?;
    let version = u32::from_be_bytes(version_bytes);
    if version != PROTOCOL_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported protocol version: {version}"),
        ));
    }

    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    if len > 10 * 1024 * 1024 {
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

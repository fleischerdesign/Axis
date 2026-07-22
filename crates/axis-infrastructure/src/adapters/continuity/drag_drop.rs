use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use axis_domain::models::continuity::Message;
use log::info;

use super::connection::ConnectionProvider;

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB

#[derive(Debug, Error)]
pub enum DragDropError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Transfer not found: {0}")]
    TransferNotFound(String),
}

pub struct ActiveIncomingTransfer {
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub destination_path: PathBuf,
    pub file: File,
    pub bytes_received: u64,
}

#[derive(Default, Clone)]
pub struct DragDropManager {
    incoming_transfers: Arc<Mutex<HashMap<String, ActiveIncomingTransfer>>>,
}

impl DragDropManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Asynchronously streams a file over the Continuity connection in 64KB chunks.
    pub async fn send_file<P: AsRef<Path>, C: ConnectionProvider>(
        &self,
        file_path: P,
        transfer_id: String,
        mime_type: String,
        connection: &C,
    ) -> Result<(), DragDropError> {
        let path = file_path.as_ref();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        let metadata = tokio::fs::metadata(path).await?;
        let is_directory = metadata.is_dir();
        let file_size = metadata.len();

        info!(
            "[continuity-dragdrop] offering file transfer {}: {} (dir={}, {} bytes)",
            transfer_id, file_name, is_directory, file_size
        );

        connection.send_message(Message::DragOffer {
            transfer_id: transfer_id.clone(),
            file_name: file_name.clone(),
            file_size,
            mime_type,
            is_directory,
            item_count: 1,
        });

        let mut file = File::open(path).await?;
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut chunk_index = 0u32;
        let mut total_sent = 0u64;

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            total_sent += bytes_read as u64;
            let is_last = total_sent >= file_size;

            connection.send_message(Message::DragChunk {
                transfer_id: transfer_id.clone(),
                chunk_index,
                is_last,
                data: buffer[..bytes_read].to_vec(),
            });

            chunk_index += 1;
            if is_last {
                break;
            }
        }

        info!(
            "[continuity-dragdrop] finished streaming transfer {}: {} chunks, {} bytes",
            transfer_id, chunk_index, total_sent
        );

        Ok(())
    }

    /// Initializes a new incoming transfer.
    pub async fn handle_offer(
        &self,
        transfer_id: String,
        file_name: String,
        file_size: u64,
        mime_type: String,
    ) -> Result<PathBuf, DragDropError> {
        let tmp_dir = std::env::temp_dir()
            .join("axis_drag_drop")
            .join(&transfer_id);
        tokio::fs::create_dir_all(&tmp_dir).await?;

        let destination_path = tmp_dir.join(&file_name);
        let file = File::create(&destination_path).await?;

        info!(
            "[continuity-dragdrop] receiving offer {}: saving to {:?}",
            transfer_id, destination_path
        );

        let active = ActiveIncomingTransfer {
            file_name,
            file_size,
            mime_type,
            destination_path: destination_path.clone(),
            file,
            bytes_received: 0,
        };

        self.incoming_transfers
            .lock()
            .await
            .insert(transfer_id, active);

        Ok(destination_path)
    }

    /// Appends an incoming chunk to the file. Returns `Some(PathBuf)` when transfer completes.
    pub async fn handle_chunk(
        &self,
        transfer_id: &str,
        _chunk_index: u32,
        is_last: bool,
        data: &[u8],
    ) -> Result<Option<PathBuf>, DragDropError> {
        let mut lock = self.incoming_transfers.lock().await;
        let transfer = lock
            .get_mut(transfer_id)
            .ok_or_else(|| DragDropError::TransferNotFound(transfer_id.to_string()))?;

        transfer.file.write_all(data).await?;
        transfer.bytes_received += data.len() as u64;

        if is_last || transfer.bytes_received >= transfer.file_size {
            transfer.file.flush().await?;
            let completed_path = transfer.destination_path.clone();
            let total_bytes = transfer.bytes_received;
            lock.remove(transfer_id);

            info!(
                "[continuity-dragdrop] transfer {} completed: {:?} ({} bytes)",
                transfer_id, completed_path, total_bytes
            );

            Ok(Some(completed_path))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    fn test_drag_drop_manager_creation() {
        let mgr = DragDropManager::new();
        assert!(mgr.incoming_transfers.blocking_lock().is_empty());
    }
}

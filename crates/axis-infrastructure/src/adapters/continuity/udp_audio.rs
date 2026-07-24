//! Dual Transport UDP Audio Server & AEAD Encrypted Datagram Protocol.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use log::{error, info};

use super::crypto::ContinuityCipher;

pub const DEFAULT_UDP_AUDIO_PORT: u16 = 15001;

pub struct UdpAudioDatagram {
    pub nonce: [u8; 12],
    pub sequence: u32,
    pub timestamp: u32,
    pub ciphertext: Vec<u8>,
}

impl UdpAudioDatagram {
    pub const MIN_DATAGRAM_LEN: usize = 20;

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::MIN_DATAGRAM_LEN + self.ciphertext.len());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.sequence.to_be_bytes());
        out.extend_from_slice(&self.timestamp.to_be_bytes());
        out.extend_from_slice(&self.ciphertext);
        out
    }

    pub fn decode(input: &[u8]) -> Option<Self> {
        if input.len() < Self::MIN_DATAGRAM_LEN {
            return None;
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&input[0..12]);
        let sequence = u32::from_be_bytes([input[12], input[13], input[14], input[15]]);
        let timestamp = u32::from_be_bytes([input[16], input[17], input[18], input[19]]);
        let ciphertext = input[20..].to_vec();
        Some(Self {
            nonce,
            sequence,
            timestamp,
            ciphertext,
        })
    }
}

pub struct UdpAudioSocket {
    socket: Arc<UdpSocket>,
    peer_addr: Arc<Mutex<Option<SocketAddr>>>,
    last_datagram_received: Arc<Mutex<Option<Instant>>>,
}

impl UdpAudioSocket {
    pub async fn bind(local_port: u16) -> Result<Self, std::io::Error> {
        let addr = format!("[::]:{local_port}");
        let socket = UdpSocket::bind(&addr).await?;
        info!("[continuity-udp-audio] bound UDP audio socket on {addr}");
        Ok(Self {
            socket: Arc::new(socket),
            peer_addr: Arc::new(Mutex::new(None)),
            last_datagram_received: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn set_peer(&self, peer: SocketAddr) {
        let mut lock = self.peer_addr.lock().await;
        *lock = Some(peer);
    }

    pub async fn send_audio_chunk(
        &self,
        chunk: &[u8],
        cipher: &Arc<std::sync::Mutex<Option<ContinuityCipher>>>,
    ) -> bool {
        let peer = {
            let lock = self.peer_addr.lock().await;
            *lock
        };
        let Some(target) = peer else {
            return false;
        };

        let encrypted = {
            let mut guard = cipher.lock().unwrap();
            if let Some(ref mut c) = *guard {
                c.encrypt(chunk)
            } else {
                chunk.to_vec()
            }
        };

        let datagram = UdpAudioDatagram {
            nonce: [0u8; 12],
            sequence: 0,
            timestamp: 0,
            ciphertext: encrypted,
        };

        let payload = datagram.encode();
        self.socket.send_to(&payload, target).await.is_ok()
    }

    pub fn spawn_receiver(
        &self,
        cipher: Arc<std::sync::Mutex<Option<ContinuityCipher>>>,
        tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) -> tokio::task::JoinHandle<()> {
        let socket = Arc::clone(&self.socket);
        let last_rcv = Arc::clone(&self.last_datagram_received);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((n, _src)) => {
                        {
                            let mut lock = last_rcv.lock().await;
                            *lock = Some(Instant::now());
                        }
                        if let Some(datagram) = UdpAudioDatagram::decode(&buf[..n]) {
                            let decrypted = {
                                let mut guard = cipher.lock().unwrap();
                                if let Some(ref mut c) = *guard {
                                    c.decrypt(&datagram.ciphertext).ok()
                                } else {
                                    Some(datagram.ciphertext)
                                }
                            };
                            if let Some(pcm) = decrypted
                                && tx.try_send(pcm).is_err()
                                && tx.is_closed()
                            {
                                break;
                            }

                        }
                    }
                    Err(e) => {
                        error!("[continuity-udp-audio] recv_from error: {e}");
                        break;
                    }
                }
            }
        })
    }

    pub async fn is_fallback_needed(&self, timeout: Duration) -> bool {
        let lock = self.last_datagram_received.lock().await;
        if let Some(last) = *lock {
            last.elapsed() > timeout
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_datagram_encode_decode() {
        let datagram = UdpAudioDatagram {
            nonce: [1u8; 12],
            sequence: 100,
            timestamp: 5000,
            ciphertext: vec![10, 20, 30],
        };
        let encoded = datagram.encode();
        let decoded = UdpAudioDatagram::decode(&encoded).unwrap();

        assert_eq!(decoded.nonce, [1u8; 12]);
        assert_eq!(decoded.sequence, 100);
        assert_eq!(decoded.timestamp, 5000);
        assert_eq!(decoded.ciphertext, vec![10, 20, 30]);
    }
}

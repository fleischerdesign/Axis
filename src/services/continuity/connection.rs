use async_channel::Sender;
use log::{error, info, warn};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use super::protocol::{self, Message};

// ── Connection Events ──────────────────────────────────────────────────

#[derive(Debug)]
pub enum ConnectionEvent {
    IncomingConnection {
        addr: std::net::SocketAddr,
        write_tx: tokio::sync::mpsc::Sender<Message>,
    },
    HandshakeComplete {
        peer_id: String,
        peer_name: String,
    },
    Disconnected {
        reason: String,
    },
    MessageReceived(Message),
    Error(String),
}

// ── Connection Provider Trait ──────────────────────────────────────────

pub trait ConnectionProvider: Send {
    fn listen(&mut self, port: u16, tx: Sender<ConnectionEvent>) -> Result<(), String>;
    fn connect_dual(
        &mut self,
        addr_v4: SocketAddr,
        addr_v6: Option<SocketAddr>,
        tx: Sender<ConnectionEvent>,
        device_id: String,
        device_name: String,
    );
    fn disconnect_active(&mut self);
    fn stop(&mut self);
    fn send_message(&self, msg: Message);
    fn set_active_write(&mut self, write_tx: tokio::sync::mpsc::Sender<Message>);
}

// ── Active Connection State ────────────────────────────────────────────

struct ActiveConnection {
    write_tx: tokio::sync::mpsc::Sender<Message>,
    task: tokio::task::JoinHandle<()>,
}

// ── TCP Implementation ─────────────────────────────────────────────────

pub struct TcpConnectionProvider {
    listen_task: Option<tokio::task::JoinHandle<()>>,
    active: Option<ActiveConnection>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl TcpConnectionProvider {
    pub fn new() -> Self {
        Self {
            listen_task: None,
            active: None,
            stop_tx: None,
        }
    }
}

impl ConnectionProvider for TcpConnectionProvider {
    fn listen(&mut self, port: u16, tx: Sender<ConnectionEvent>) -> Result<(), String> {
        let (stop_tx, stop_rx) = oneshot::channel();
        self.stop_tx = Some(stop_tx);

        let task = tokio::spawn(async move {
            if let Err(e) = listen_loop(port, tx, stop_rx).await {
                error!("[continuity:connection] listen error: {e}");
            }
        });

        self.listen_task = Some(task);
        Ok(())
    }

    fn connect_dual(
        &mut self,
        addr_v4: SocketAddr,
        addr_v6: Option<SocketAddr>,
        tx: Sender<ConnectionEvent>,
        device_id: String,
        device_name: String,
    ) {
        self.disconnect_active();

        let (write_tx, write_rx) = tokio::sync::mpsc::channel::<Message>(64);

        let task = tokio::spawn(async move {
            // Try IPv6 first if available (3s timeout)
            if let Some(v6) = addr_v6 {
                info!("[continuity:connection] trying IPv6 {v6}");
                match tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    TcpStream::connect(v6),
                )
                .await
                {
                    Ok(Ok(stream)) => {
                        info!("[continuity:connection] connected via IPv6 to {v6}");
                        run_connection(stream, write_rx, tx, true, device_id, device_name).await;
                        return;
                    }
                    Ok(Err(e)) => {
                        warn!("[continuity:connection] IPv6 failed: {e}");
                    }
                    Err(_) => {
                        warn!("[continuity:connection] IPv6 timed out");
                    }
                }
            }

            // Fallback to IPv4 (5s timeout)
            info!("[continuity:connection] trying IPv4 {addr_v4}");
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                TcpStream::connect(addr_v4),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    info!("[continuity:connection] connected via IPv4 to {addr_v4}");
                    run_connection(stream, write_rx, tx, true, device_id, device_name).await;
                }
                Ok(Err(e)) => {
                    error!("[continuity:connection] IPv4 failed: {e}");
                    let _ = tx.send(ConnectionEvent::Error(e.to_string())).await;
                }
                Err(_) => {
                    error!("[continuity:connection] IPv4 timed out");
                    let _ = tx
                        .send(ConnectionEvent::Error("connection timed out".into()))
                        .await;
                }
            }
        });

        self.active = Some(ActiveConnection { write_tx, task });
    }

    fn disconnect_active(&mut self) {
        if let Some(conn) = self.active.take() {
            conn.task.abort();
        }
    }

    fn stop(&mut self) {
        self.disconnect_active();

        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(task) = self.listen_task.take() {
            task.abort();
        }
    }

    fn send_message(&self, msg: Message) {
        if let Some(conn) = &self.active {
            let _ = conn.write_tx.try_send(msg);
        }
    }

    fn set_active_write(&mut self, write_tx: tokio::sync::mpsc::Sender<Message>) {
        self.active = Some(ActiveConnection {
            write_tx,
            task: tokio::spawn(async {}),
        });
    }
}

impl Drop for TcpConnectionProvider {
    fn drop(&mut self) {
        self.stop();
    }
}

// ── Internal Helpers ───────────────────────────────────────────────────

async fn listen_loop(
    port: u16,
    event_tx: Sender<ConnectionEvent>,
    stop_rx: oneshot::Receiver<()>,
) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    info!("[continuity:connection] listening on port {port}");

    tokio::pin!(let stop = stop_rx;);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        info!("[continuity:connection] incoming from {addr}");
                        let tx = event_tx.clone();
                        let (write_tx, write_rx) =
                            tokio::sync::mpsc::channel::<Message>(64);

                        // Send IncomingConnection with write channel — service uses this to send messages
                        let _ = tx.send(ConnectionEvent::IncomingConnection {
                            addr,
                            write_tx,
                        }).await;

                        // Start reading from the stream — service sends messages via write_rx
                        tokio::spawn(async move {
                            run_connection(stream, write_rx, tx, false, String::new(), String::new()).await;
                        });
                    }
                    Err(e) => {
                        warn!("[continuity:connection] accept error: {e}");
                    }
                }
            }
            _ = &mut stop => {
                info!("[continuity:connection] listener stopped");
                break;
            }
        }
    }

    Ok(())
}

async fn run_connection(
    stream: TcpStream,
    mut write_rx: tokio::sync::mpsc::Receiver<Message>,
    event_tx: Sender<ConnectionEvent>,
    is_initiator: bool,
    device_id: String,
    device_name: String,
) {
    use tokio::io::split;
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    let local = stream.local_addr().map(|a| a.to_string()).unwrap_or_default();
    info!(
        "[continuity:connection] run_connection: local={local} peer={peer} initiator={is_initiator}"
    );

    let (mut reader, mut writer) = split(stream);

    // If we initiated the connection, send Hello first
    if is_initiator {
        info!("[continuity:connection] sending Hello as initiator");
        let hello = Message::Hello {
            device_id,
            device_name,
            version: protocol::PROTOCOL_VERSION,
        };
        if let Err(e) = protocol::write_message(&mut writer, &hello).await {
            error!("[continuity:connection] send hello failed: {e}");
            return;
        }
    } else {
        info!("[continuity:connection] waiting for Hello from initiator ({peer})");
    }

    info!("[continuity:connection] entering message loop ({peer})");
    loop {
        tokio::select! {
            result = protocol::read_message(&mut reader) => {
                match result {
                    Ok(msg) => {
                        let _ = event_tx.send(ConnectionEvent::MessageReceived(msg)).await;
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::UnexpectedEof {
                            info!("[continuity:connection] peer disconnected");
                            let _ = event_tx
                                .send(ConnectionEvent::Disconnected {
                                    reason: "peer disconnected".into(),
                                })
                                .await;
                        } else {
                            error!("[continuity:connection] read error: {e}");
                            let _ = event_tx
                                .send(ConnectionEvent::Error(e.to_string()))
                                .await;
                        }
                        break;
                    }
                }
            }
            Some(msg) = write_rx.recv() => {
                if let Err(e) = protocol::write_message(&mut writer, &msg).await {
                    error!("[continuity:connection] write error: {e}");
                    break;
                }
            }
            else => break,
        }
    }
}

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
        peer_name: String,
        stream: TcpStream,
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
    fn connect(
        &mut self,
        addr: SocketAddr,
        tx: Sender<ConnectionEvent>,
    ) -> Result<(), String>;
    fn connect_dual(
        &mut self,
        addr_v4: SocketAddr,
        addr_v6: Option<SocketAddr>,
        tx: Sender<ConnectionEvent>,
    );
    fn disconnect_active(&mut self);
    fn stop(&mut self);
    fn send_message(&self, msg: Message);
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

    fn connect(
        &mut self,
        addr: SocketAddr,
        tx: Sender<ConnectionEvent>,
    ) -> Result<(), String> {
        // Disconnect any existing connection first
        self.disconnect_active();

        let (write_tx, write_rx) = tokio::sync::mpsc::channel::<Message>(64);

        let task = tokio::spawn(async move {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                TcpStream::connect(addr),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    info!("[continuity:connection] connected to {addr}");
                    run_connection(stream, write_rx, tx, true).await;
                }
                Ok(Err(e)) => {
                    error!("[continuity:connection] connect to {addr} failed: {e}");
                    let _ = tx.send(ConnectionEvent::Error(e.to_string())).await;
                }
                Err(_) => {
                    error!("[continuity:connection] connect to {addr} timed out");
                    let _ = tx
                        .send(ConnectionEvent::Error("connection timed out".into()))
                        .await;
                }
            }
        });

        self.active = Some(ActiveConnection {
            write_tx,
            task,
        });

        Ok(())
    }

    fn connect_dual(
        &mut self,
        addr_v4: SocketAddr,
        addr_v6: Option<SocketAddr>,
        tx: Sender<ConnectionEvent>,
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
                        run_connection(stream, write_rx, tx, true).await;
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
                    run_connection(stream, write_rx, tx, true).await;
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

        self.active = Some(ActiveConnection {
            write_tx,
            task,
        });
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
                        tokio::spawn(async move {
                            // Start handshake as server side
                            let (_write_tx, write_rx) =
                                tokio::sync::mpsc::channel::<Message>(64);
                            run_connection(stream, write_rx, tx, false).await;
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
) {
    use tokio::io::split;
    let (mut reader, mut writer) = split(stream);

    // If we initiated the connection, send Hello first
    if is_initiator {
        let hello = Message::Hello {
            device_id: String::new(), // TODO: pass device_id
            device_name: String::new(), // TODO: pass device_name
            version: protocol::PROTOCOL_VERSION,
        };
        if let Err(e) = protocol::write_message(&mut writer, &hello).await {
            error!("[continuity:connection] send hello failed: {e}");
            return;
        }
    }

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

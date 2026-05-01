use async_channel::Sender;
use axis_domain::models::continuity::Message;
use log::{debug, error, info, warn};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use super::proto;

#[derive(Debug)]
pub enum ConnectionEvent {
    IncomingConnection {
        addr: SocketAddr,
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

struct ActiveConnection {
    write_tx: tokio::sync::mpsc::Sender<Message>,
    task: tokio::task::JoinHandle<()>,
}

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
        let task = self
            .active
            .take()
            .map(|c| c.task)
            .unwrap_or_else(|| tokio::spawn(async {}));
        self.active = Some(ActiveConnection { write_tx, task });
    }
}

impl Drop for TcpConnectionProvider {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn listen_loop(
    port: u16,
    event_tx: Sender<ConnectionEvent>,
    stop_rx: oneshot::Receiver<()>,
) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(format!("[::]:{port}")).await?;
    info!("[continuity:connection] listening on port {port}");

    tokio::pin!(let stop = stop_rx;);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        debug!("[continuity:connection] incoming from {addr}");
                        let tx = event_tx.clone();
                        let (write_tx, write_rx) =
                            tokio::sync::mpsc::channel::<Message>(64);

                        let _ = tx.send(ConnectionEvent::IncomingConnection {
                            addr,
                            write_tx,
                        }).await;

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
    if let Err(e) = stream.set_nodelay(true) {
        warn!("[continuity:connection] failed to set TCP_NODELAY: {e}");
    }

    use tokio::io::split;
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    let local = stream.local_addr().map(|a| a.to_string()).unwrap_or_default();
    info!(
        "[continuity:connection] connected {local} → {peer}"
    );

    let (mut reader, mut writer) = split(stream);

    if is_initiator {
        let hello = Message::Hello {
            device_id,
            device_name,
            version: proto::PROTOCOL_VERSION,
        };
        if let Err(e) = proto::write_message(&mut writer, &hello).await {
            error!("[continuity:connection] send hello failed: {e}");
            return;
        }
    } else {
        debug!("[continuity:connection] waiting for Hello from {peer}");
    }

    debug!("[continuity:connection] message loop started ({peer})");
    loop {
        tokio::select! {
            result = proto::read_message(&mut reader) => {
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
                if let Err(e) = proto::write_message(&mut writer, &msg).await {
                    error!("[continuity:connection] write error: {e}");
                    break;
                }
            }
            else => break,
        }
    }
}

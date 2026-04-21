use axis_domain::models::lock::LockStatus;
use axis_domain::ports::lock::{LockProvider, LockError, LockStream};
use async_trait::async_trait;
use gtk4::prelude::*;
use gtk4_session_lock as session_lock;
use log::{error, info, warn};
use tokio::sync::{watch, mpsc, oneshot};
use tokio_stream::wrappers::WatchStream;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

enum LockCommand {
    Lock(oneshot::Sender<Result<(), LockError>>),
    Unlock(oneshot::Sender<Result<(), LockError>>),
    Authenticate { password: String, reply: oneshot::Sender<Result<bool, LockError>> },
}

pub struct SessionLockProvider {
    status_tx: watch::Sender<LockStatus>,
    cmd_tx: mpsc::Sender<LockCommand>,
}

pub struct LockGtkHandle {
    inner: Rc<RefCell<LockAdapterInner>>,
}

impl LockGtkHandle {
    pub fn set_content_factory(&self, factory: Box<dyn Fn() -> gtk4::Widget>) {
        self.inner.borrow_mut().content_factory = Some(factory);
    }
}

impl SessionLockProvider {
    pub fn new() -> (Arc<Self>, LockGtkHandle) {
        let initial = LockStatus {
            is_locked: false,
            is_supported: false,
        };
        let (status_tx, _) = watch::channel(initial);
        let (cmd_tx, cmd_rx) = mpsc::channel(16);

        let provider = Arc::new(Self { status_tx: status_tx.clone(), cmd_tx: cmd_tx.clone() });

        let inner = Rc::new(RefCell::new(LockAdapterInner {
            status_tx: status_tx.clone(),
            instance: None,
            windows: Vec::new(),
            locked: false,
            lock_confirmed: false,
            pending_unlock: false,
            content_factory: None,
        }));

        let handle = LockGtkHandle { inner: inner.clone() };

        gtk4::glib::idle_add_local_once(move || {
            let supported = session_lock::is_supported();
            if supported {
                info!("[lock] Session lock protocol supported");
            } else {
                warn!("[lock] Session lock protocol NOT supported");
            }
            let _ = status_tx.send(LockStatus {
                is_locked: false,
                is_supported: supported,
            });

            gtk4::glib::spawn_future_local(async move {
                LockAdapterInner::run(inner, cmd_rx).await;
            });
        });

        let status_tx_c = provider.status_tx.clone();
        let cmd_tx_c = provider.cmd_tx.clone();
        tokio::spawn(async move {
            listen_logind_signals(status_tx_c, cmd_tx_c).await;
        });

        (provider, handle)
    }
}

#[async_trait]
impl LockProvider for SessionLockProvider {
    async fn is_supported(&self) -> Result<bool, LockError> {
        Ok(self.status_tx.borrow().is_supported)
    }

    async fn lock(&self) -> Result<(), LockError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(LockCommand::Lock(tx)).await
            .map_err(|e| LockError::ProviderError(e.to_string()))?;
        rx.await.map_err(|e| LockError::ProviderError(e.to_string()))?
    }

    async fn unlock(&self) -> Result<(), LockError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(LockCommand::Unlock(tx)).await
            .map_err(|e| LockError::ProviderError(e.to_string()))?;
        rx.await.map_err(|e| LockError::ProviderError(e.to_string()))?
    }

    async fn authenticate(&self, password: &str) -> Result<bool, LockError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(LockCommand::Authenticate { password: password.to_string(), reply: tx }).await
            .map_err(|e| LockError::ProviderError(e.to_string()))?;
        rx.await.map_err(|e| LockError::ProviderError(e.to_string()))?
    }

    async fn subscribe(&self) -> Result<LockStream, LockError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }
}

struct LockAdapterInner {
    status_tx: watch::Sender<LockStatus>,
    instance: Option<session_lock::Instance>,
    windows: Vec<gtk4::Window>,
    locked: bool,
    lock_confirmed: bool,
    pending_unlock: bool,
    content_factory: Option<Box<dyn Fn() -> gtk4::Widget>>,
}

impl LockAdapterInner {
    async fn run(adapter: Rc<RefCell<Self>>, mut cmd_rx: mpsc::Receiver<LockCommand>) {
        loop {
            let cmd = cmd_rx.recv().await;

            match cmd {
                Some(LockCommand::Lock(reply)) => {
                    let result = Self::do_lock(&adapter);
                    let _ = reply.send(result);
                }
                Some(LockCommand::Unlock(reply)) => {
                    let result = Self::do_unlock(&adapter);
                    let _ = reply.send(result);
                }
                Some(LockCommand::Authenticate { password, reply }) => {
                    let result = Self::do_authenticate(&password);
                    let _ = reply.send(Ok(result));
                }
                None => break,
            }
        }
    }

    fn do_lock(adapter: &Rc<RefCell<Self>>) -> Result<(), LockError> {
        let locked = adapter.borrow().locked;
        if locked {
            return Ok(());
        }

        if !session_lock::is_supported() {
            return Err(LockError::NotSupported);
        }

        info!("[lock] Locking session");

        let instance = session_lock::Instance::new();

        let adapter_c = adapter.clone();
        instance.connect_locked(move |_| {
            info!("[lock] Session locked by compositor");
            let mut borrowed = adapter_c.borrow_mut();
            borrowed.lock_confirmed = true;
            if borrowed.pending_unlock {
                info!("[lock] Deferred unlock executing now");
                Self::perform_unlock_inner(&mut borrowed);
            }
        });

        let adapter_c = adapter.clone();
        instance.connect_failed(move |_| {
            warn!("[lock] Lock failed — another locker holds the lock");
            let mut borrowed = adapter_c.borrow_mut();
            borrowed.instance = None;
            borrowed.windows.clear();
            let _ = borrowed.status_tx.send(LockStatus {
                is_locked: false,
                is_supported: true,
            });
        });

        if !instance.lock() {
            error!("[lock] Failed to acquire lock (immediate failure)");
            return Err(LockError::ProviderError("Failed to acquire lock".to_string()));
        }

        let display = gtk4::gdk::Display::default().expect("No display available");
        let monitors = display.monitors();

        {
            let mut borrowed = adapter.borrow_mut();
            borrowed.windows.clear();
            for i in 0..monitors.n_items() {
                let Some(monitor_obj) = monitors.item(i) else { continue };
                let Some(monitor) = monitor_obj.downcast_ref::<gtk4::gdk::Monitor>() else { continue };

                let content = borrowed.content_factory.as_ref().map(|f| f());

                let window = gtk4::Window::builder()
                    .title("Lock Screen")
                    .build();

                if let Some(content) = content {
                    window.set_child(Some(&content));
                }

                instance.assign_window_to_monitor(&window, monitor);
                window.present();
                borrowed.windows.push(window);
            }
            borrowed.instance = Some(instance);
            borrowed.locked = true;
        }

        {
            let borrowed = adapter.borrow();
            let _ = borrowed.status_tx.send(LockStatus {
                is_locked: true,
                is_supported: true,
            });
        }

        Ok(())
    }

    fn do_unlock(adapter: &Rc<RefCell<Self>>) -> Result<(), LockError> {
        let mut borrowed = adapter.borrow_mut();
        if !borrowed.locked {
            return Ok(());
        }

        if borrowed.lock_confirmed {
            Self::perform_unlock_inner(&mut borrowed);
        } else {
            info!("[lock] Deferring unlock until compositor confirms lock");
            borrowed.pending_unlock = true;
        }

        Ok(())
    }

    fn perform_unlock_inner(borrowed: &mut Self) {
        if let Some(inst) = borrowed.instance.take() {
            info!("[lock] Unlocking session");
            inst.unlock();
            if let Some(display) = gtk4::gdk::Display::default() {
                display.sync();
            }
        }
        borrowed.windows.clear();
        borrowed.locked = false;
        borrowed.lock_confirmed = false;
        borrowed.pending_unlock = false;
        let _ = borrowed.status_tx.send(LockStatus {
            is_locked: false,
            is_supported: true,
        });
    }

    fn do_authenticate(password: &str) -> bool {
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "user".into());

        let mut client = match pam::Client::with_password("login") {
            Ok(c) => c,
            Err(e) => {
                error!("[lock] PAM init failed: {e}");
                return false;
            }
        };

        client
            .conversation_mut()
            .set_credentials(&username, password);

        match client.authenticate() {
            Ok(()) => true,
            Err(e) => {
                warn!("[lock] PAM auth failed: {e}");
                false
            }
        }
    }
}

async fn listen_logind_signals(
    status_tx: watch::Sender<LockStatus>,
    cmd_tx: mpsc::Sender<LockCommand>,
) {
    use futures_util::StreamExt;

    let conn = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            warn!("[lock] Failed to connect to system D-Bus for logind signals: {e}");
            return;
        }
    };

    let manager_proxy = match zbus::Proxy::new(
        &conn,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            warn!("[lock] Failed to create logind manager proxy: {e}");
            return;
        }
    };

    let mut sleep_stream = match manager_proxy.receive_signal("PrepareForSleep").await {
        Ok(s) => s,
        Err(e) => {
            warn!("[lock] Failed to subscribe to PrepareForSleep signal: {e}");
            return;
        }
    };

    let session_path: Option<zbus::zvariant::OwnedObjectPath> =
        manager_proxy.get_property("Session").await.ok();

    let mut lock_stream_opt = if let Some(ref path) = session_path {
        match zbus::Proxy::new(
            &conn,
            "org.freedesktop.login1",
            path.as_str(),
            "org.freedesktop.login1.Session",
        )
        .await
        {
            Ok(session_proxy) => session_proxy.receive_signal("Lock").await.ok(),
            Err(e) => {
                warn!("[lock] Failed to create session proxy: {e}");
                None
            }
        }
    } else {
        warn!("[lock] Could not determine session path");
        None
    };

    info!("[lock] Listening for logind signals");

    loop {
        if let Some(ref mut lock_stream) = lock_stream_opt {
            tokio::select! {
                Some(msg) = sleep_stream.next() => {
                    if let Ok(sleeping) = msg.body().deserialize::<bool>() {
                        if sleeping && !status_tx.borrow().is_locked {
                            let (tx, rx) = oneshot::channel();
                            let _ = cmd_tx.send(LockCommand::Lock(tx)).await;
                            let _ = rx.await;
                        }
                    }
                }
                Some(_msg) = lock_stream.next() => {
                    if !status_tx.borrow().is_locked {
                        let (tx, rx) = oneshot::channel();
                        let _ = cmd_tx.send(LockCommand::Lock(tx)).await;
                        let _ = rx.await;
                    }
                }
            }
        } else {
            if let Some(msg) = sleep_stream.next().await {
                if let Ok(sleeping) = msg.body().deserialize::<bool>() {
                    if sleeping && !status_tx.borrow().is_locked {
                        let (tx, rx) = oneshot::channel();
                        let _ = cmd_tx.send(LockCommand::Lock(tx)).await;
                        let _ = rx.await;
                    }
                }
            }
        }
    }
}

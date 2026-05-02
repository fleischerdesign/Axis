use axis_domain::models::lock::LockStatus;
use axis_domain::ports::lock::{LockProvider, LockError, LockStream};
use axis_domain::ports::idle_inhibit::IdleInhibitProvider;
use axis_domain::ports::config::ConfigProvider;
use async_trait::async_trait;
use gtk4::prelude::*;
use gtk4_session_lock as session_lock;
use log::{error, info, warn};
use tokio::sync::{watch, mpsc, oneshot};
use tokio::time::Duration;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::WatchStream;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

enum LockCommand {
    Lock(oneshot::Sender<Result<(), LockError>>),
    Unlock(oneshot::Sender<Result<(), LockError>>),
    Authenticate { password: String, reply: oneshot::Sender<Result<bool, LockError>> },
}

pub struct SessionLockProvider {
    status_tx: watch::Sender<LockStatus>,
    cmd_tx: mpsc::Sender<LockCommand>,
    #[allow(dead_code)]
    idle_inhibit_provider: Arc<dyn IdleInhibitProvider>,
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
    pub fn new(
        idle_inhibit_provider: Arc<dyn IdleInhibitProvider>,
        config_provider: Arc<dyn ConfigProvider>,
        idle_lock_timeout_seconds: Option<u32>,
        idle_blank_timeout_seconds: Option<u32>,
    ) -> (Arc<Self>, LockGtkHandle) {
        let initial = LockStatus {
            is_locked: false,
            is_supported: false,
        };
        let (status_tx, _) = watch::channel(initial);
        let (cmd_tx, cmd_rx) = mpsc::channel(16);

        let provider = Arc::new(Self {
            status_tx: status_tx.clone(),
            cmd_tx: cmd_tx.clone(),
            idle_inhibit_provider: idle_inhibit_provider.clone(),
        });

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
        let iip = idle_inhibit_provider;
        tokio::spawn(async move {
            listen_logind_signals(
                status_tx_c,
                cmd_tx_c,
                iip,
                config_provider,
                idle_lock_timeout_seconds,
                idle_blank_timeout_seconds,
            ).await;
        });

        (provider, handle)
    }
}

#[async_trait]
impl LockProvider for SessionLockProvider {
    async fn get_status(&self) -> Result<LockStatus, LockError> {
        Ok(self.status_tx.borrow().clone())
    }

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

        let Some(display) = gtk4::gdk::Display::default() else {
            return Err(LockError::ProviderError("No display available".to_string()));
        };
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
    idle_inhibit_provider: Arc<dyn IdleInhibitProvider>,
    config_provider: Arc<dyn ConfigProvider>,
    idle_lock_timeout_seconds: Option<u32>,
    idle_blank_timeout_seconds: Option<u32>,
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

    let session_path = resolve_session_path(&manager_proxy).await;

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

    spawn_idle_monitor(
        cmd_tx.clone(),
        idle_inhibit_provider.clone(),
        config_provider,
        idle_lock_timeout_seconds,
        idle_blank_timeout_seconds,
    );

    info!("[lock] Listening for logind signals");

    loop {
        if let Some(ref mut lock_stream) = lock_stream_opt {
            tokio::select! {
                Some(msg) = sleep_stream.next() => {
                    if let Ok(sleeping) = msg.body().deserialize::<bool>() {
                        if sleeping && !status_tx.borrow().is_locked {
                            if is_inhibited(&idle_inhibit_provider).await {
                                info!("[lock] Idle inhibit active, skipping PrepareForSleep lock");
                                continue;
                            }
                            let (tx, rx) = oneshot::channel();
                            let _ = cmd_tx.send(LockCommand::Lock(tx)).await;
                            let _ = rx.await;
                        }
                    }
                }
                Some(_msg) = lock_stream.next() => {
                    if !status_tx.borrow().is_locked {
                        if is_inhibited(&idle_inhibit_provider).await {
                            info!("[lock] Idle inhibit active, skipping Session.Lock lock");
                            continue;
                        }
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
                        if is_inhibited(&idle_inhibit_provider).await {
                            info!("[lock] Idle inhibit active, skipping PrepareForSleep lock");
                            continue;
                        }
                        let (tx, rx) = oneshot::channel();
                        let _ = cmd_tx.send(LockCommand::Lock(tx)).await;
                        let _ = rx.await;
                    }
                }
            }
        }
    }
}

async fn resolve_session_path(
    manager_proxy: &zbus::Proxy<'_>,
) -> Option<zbus::zvariant::OwnedObjectPath> {
    if let Ok(session_id) = std::env::var("XDG_SESSION_ID") {
        match manager_proxy
            .call::<_, _, zbus::zvariant::OwnedObjectPath>("GetSession", &(session_id.as_str(),))
            .await
        {
            Ok(path) => {
                info!("[lock] Session resolved via GetSession({session_id})");
                return Some(path);
            }
            Err(e) => {
                warn!("[lock] GetSession({session_id}) failed: {e}");
            }
        }
    }

    let pid = std::process::id();
    match manager_proxy
        .call::<_, _, (String, zbus::zvariant::OwnedObjectPath)>(
            "GetSessionByPID",
            &(pid,),
        )
        .await
    {
        Ok((_id, path)) => {
            info!("[lock] Session resolved via GetSessionByPID");
            Some(path)
        }
        Err(e) => {
            warn!("[lock] GetSessionByPID failed: {e}");
            None
        }
    }
}

fn spawn_idle_monitor(
    cmd_tx: mpsc::Sender<LockCommand>,
    idle_inhibit_provider: Arc<dyn IdleInhibitProvider>,
    config_provider: Arc<dyn ConfigProvider>,
    lock_timeout_seconds: Option<u32>,
    blank_timeout_seconds: Option<u32>,
) {
    if lock_timeout_seconds.is_none() && blank_timeout_seconds.is_none() {
        info!("[lock] No idle timeouts configured, skipping idle monitor");
        return;
    }

    let idle_rx = match crate::adapters::idle_notify::create_idle_watcher(500) {
        Some(rx) => rx,
        None => {
            info!("[lock] Wayland idle notify not available, skipping idle monitor");
            return;
        }
    };

    tokio::spawn(async move {
        run_idle_monitor(
            idle_rx,
            cmd_tx,
            idle_inhibit_provider,
            config_provider,
            lock_timeout_seconds,
            blank_timeout_seconds,
        )
        .await;
    });
}

async fn run_idle_monitor(
    mut idle_rx: mpsc::Receiver<crate::adapters::idle_notify::IdleEvent>,
    cmd_tx: mpsc::Sender<LockCommand>,
    idle_inhibit_provider: Arc<dyn IdleInhibitProvider>,
    config_provider: Arc<dyn ConfigProvider>,
    mut lock_timeout_seconds: Option<u32>,
    mut blank_timeout_seconds: Option<u32>,
) {
    use crate::adapters::idle_notify::IdleEvent;
    use futures_util::StreamExt;

    let monitors_blanked = Arc::new(AtomicBool::new(false));
    let mut blank_handle: Option<JoinHandle<()>> = None;
    let mut lock_handle: Option<JoinHandle<()>> = None;
    let mut config_stream = match config_provider.subscribe() {
        Ok(s) => Some(s),
        Err(e) => {
            warn!("[lock] Config subscribe failed, idle timeouts will not hot-reload: {e}");
            None
        }
    };

    info!("[lock] Idle monitor started (lock_timeout={lock_timeout_seconds:?}, blank_timeout={blank_timeout_seconds:?})");

    loop {
        tokio::select! {
            event = idle_rx.recv() => {
                let Some(event) = event else { break };

                match event {
                    IdleEvent::Idled => {
                        info!("[lock] User idle, starting timers");
                        start_timers(
                            &mut blank_handle,
                            &mut lock_handle,
                            &monitors_blanked,
                            &cmd_tx,
                            &idle_inhibit_provider,
                            blank_timeout_seconds,
                            lock_timeout_seconds,
                        );
                    }
                    IdleEvent::Resumed => {
                        info!("[lock] User active, canceling timers");
                        cancel_timers(&mut blank_handle, &mut lock_handle, &monitors_blanked).await;
                    }
                }
            }
            cfg = async {
                match config_stream.as_mut() {
                    Some(s) => s.next().await,
                    None => futures_util::future::pending().await,
                }
            } => {
                if let Some(config) = cfg {
                    let new_lock = config.idle.lock_timeout_seconds;
                    let new_blank = config.idle.blank_timeout_seconds;

                    if new_lock != lock_timeout_seconds || new_blank != blank_timeout_seconds {
                        info!("[lock] Idle timeouts changed live (lock={new_lock:?}, blank={new_blank:?})");
                        lock_timeout_seconds = new_lock;
                        blank_timeout_seconds = new_blank;
                    }
                } else {
                    warn!("[lock] Config stream ended, idle timeouts will no longer hot-reload");
                    config_stream = None;
                }
            }
        }
    }
}

fn start_timers(
    blank_handle: &mut Option<JoinHandle<()>>,
    lock_handle: &mut Option<JoinHandle<()>>,
    monitors_blanked: &Arc<AtomicBool>,
    cmd_tx: &mpsc::Sender<LockCommand>,
    idle_inhibit_provider: &Arc<dyn IdleInhibitProvider>,
    blank_timeout_seconds: Option<u32>,
    lock_timeout_seconds: Option<u32>,
) {
    if let Some(secs) = blank_timeout_seconds {
        let iip = idle_inhibit_provider.clone();
        let mb = monitors_blanked.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(secs as u64)).await;
            if is_inhibited(&iip).await {
                info!("[lock] Idle inhibit active, skipping blank");
                return;
            }
            info!("[lock] Blank timeout reached, powering off monitors");
            niri_action(niri_ipc::Action::PowerOffMonitors {}).await;
            mb.store(true, Ordering::SeqCst);
        });
        *blank_handle = Some(handle);
    }

    if let Some(secs) = lock_timeout_seconds {
        let iip = idle_inhibit_provider.clone();
        let tx = cmd_tx.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(secs as u64)).await;
            if is_inhibited(&iip).await {
                info!("[lock] Idle inhibit active, skipping auto-lock");
                return;
            }
            info!("[lock] Lock timeout reached, locking session");
            let (reply_tx, reply_rx) = oneshot::channel();
            let _ = tx.send(LockCommand::Lock(reply_tx)).await;
            let _ = reply_rx.await;
        });
        *lock_handle = Some(handle);
    }
}

async fn cancel_timers(
    blank_handle: &mut Option<tokio::task::JoinHandle<()>>,
    lock_handle: &mut Option<tokio::task::JoinHandle<()>>,
    monitors_blanked: &Arc<AtomicBool>,
) {
    if let Some(h) = blank_handle.take() {
        h.abort();
    }
    if let Some(h) = lock_handle.take() {
        h.abort();
    }
    if monitors_blanked.swap(false, Ordering::SeqCst) {
        info!("[lock] Powering on monitors");
        niri_action(niri_ipc::Action::PowerOnMonitors {}).await;
    }
}

async fn is_inhibited(idle_inhibit_provider: &Arc<dyn IdleInhibitProvider>) -> bool {
    match idle_inhibit_provider.get_status().await {
        Ok(status) => status.inhibited,
        Err(e) => {
            log::warn!("[lock] failed to check idle inhibit status: {e}");
            false
        }
    }
}

async fn niri_action(action: niri_ipc::Action) {
    let result = tokio::task::spawn_blocking(move || {
        let mut sock = match niri_ipc::socket::Socket::connect() {
            Ok(s) => s,
            Err(e) => {
                log::warn!("[lock] niri connect failed: {e}");
                return;
            }
        };
        match sock.send(niri_ipc::Request::Action(action)) {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => log::warn!("[lock] niri action failed: {e}"),
            Err(e) => log::warn!("[lock] niri send failed: {e}"),
        }
    });
    let _ = result.await;
}

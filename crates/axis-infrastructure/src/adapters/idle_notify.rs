use log::{info, warn};
use tokio::sync::mpsc;
use wayland_client::{Connection, Dispatch, QueueHandle, globals::{GlobalListContents, registry_queue_init}};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

pub enum IdleEvent {
    Idled,
    Resumed,
}

struct IdleState {
    tx: mpsc::Sender<IdleEvent>,
    _notification: Option<ExtIdleNotificationV1>,
}

impl Dispatch<ExtIdleNotificationV1, ()> for IdleState {
    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                info!("[idle-notify] User idled");
                let _ = state.tx.try_send(IdleEvent::Idled);
            }
            ext_idle_notification_v1::Event::Resumed => {
                info!("[idle-notify] User resumed");
                let _ = state.tx.try_send(IdleEvent::Resumed);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for IdleState {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<wl_seat::WlSeat, ()> for IdleState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<ExtIdleNotifierV1, ()> for IdleState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtIdleNotifierV1,
        _event: <ExtIdleNotifierV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {}
}

pub fn create_idle_watcher(timeout_ms: u32) -> Option<mpsc::Receiver<IdleEvent>> {
    let (tx, rx) = mpsc::channel(1);

    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            warn!("[idle-notify] Failed to connect to Wayland: {e}");
            return None;
        }
    };

    let (globals, mut event_queue) = match registry_queue_init::<IdleState>(&conn) {
        Ok(g) => g,
        Err(e) => {
            warn!("[idle-notify] Failed to init registry: {e}");
            return None;
        }
    };
    let qh = event_queue.handle();

    let seat: wl_seat::WlSeat = match globals.bind::<wl_seat::WlSeat, _, _>(&qh, 1..=10, ()) {
        Ok(s) => s,
        Err(e) => {
            warn!("[idle-notify] No seat found: {e}");
            return None;
        }
    };

    let notifier: ExtIdleNotifierV1 = match globals.bind::<ExtIdleNotifierV1, _, _>(&qh, 1..=2, ()) {
        Ok(n) => n,
        Err(e) => {
            warn!("[idle-notify] ext_idle_notifier_v1 not available: {e}");
            return None;
        }
    };

    let notification = notifier.get_idle_notification(timeout_ms, &seat, &qh, ());

    let mut state = IdleState {
        tx,
        _notification: Some(notification),
    };

    std::thread::spawn(move || loop {
        if let Err(e) = event_queue.blocking_dispatch(&mut state) {
            warn!("[idle-notify] Dispatch stopped: {e}");
            break;
        }
    });

    info!("[idle-notify] Idle watcher started (timeout={timeout_ms}ms)");
    Some(rx)
}

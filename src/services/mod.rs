use async_channel::Sender;
use crate::store::ServiceStore;

/// Unified service trait — every service implements this.
/// Read-only services use `type Cmd = ()`.
pub trait Service: 'static {
    type Data: Clone + PartialEq + Send + 'static;
    type Cmd: Send + 'static;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>);
}

pub mod niri;
pub mod audio;
pub mod network;
pub mod bluetooth;
pub mod power;
pub mod clock;
pub mod backlight;
pub mod nightlight;
pub mod launcher;
pub mod ipc;
pub mod notifications;
pub mod dnd;
pub mod tray;
pub mod kdeconnect;

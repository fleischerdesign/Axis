use async_channel::Sender;
use crate::store::ServiceStore;

pub trait Service: 'static {
    type Data: Clone + PartialEq + Send + 'static;
    type Cmd: Send + 'static;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>);
}

pub trait ServiceConfig: Service {
    fn get_enabled(data: &Self::Data) -> bool;
    fn cmd_set_enabled(on: bool) -> Self::Cmd;
}

pub mod airplane;
pub mod audio;
pub mod backlight;
pub mod bluetooth;
pub mod clock;
pub mod dnd;
pub mod kdeconnect;
pub mod network;
pub mod nightlight;
pub mod notifications;
pub mod power;
pub mod settings;
pub mod calendar;
pub mod google;
pub mod tasks;
pub mod tray;
pub mod continuity;

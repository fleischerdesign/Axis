use async_channel::Sender;
use chrono::{DateTime, Local};

use crate::services::audio::{AudioCmd, AudioData};
use crate::services::backlight::{BacklightCmd, BacklightData};
use crate::services::bluetooth::{BluetoothCmd, BluetoothData};
use crate::services::dnd::{DndCmd, DndData};
use crate::services::launcher::{LauncherCmd, LauncherData};
use crate::services::network::{NetworkCmd, NetworkData};
use crate::services::nightlight::{NightlightCmd, NightlightData};
use crate::services::niri::NiriData;
use crate::services::notifications::{server::NotificationCmd, NotificationData};
use crate::services::power::PowerData;
use crate::services::tray::{TrayCmd, TrayData};
use crate::store::ServiceStore;

/// Zentraler App-Kontext — wird an alle Widgets weitergegeben.
///
/// Stores sind `Clone` und teilen intern denselben Zustand (via `Rc`).
/// Jedes Widget kann beliebig viele `subscribe()`-Callbacks registrieren,
/// ohne dass weitere Channels oder Receiver angelegt werden müssen.
#[derive(Clone)]
pub struct AppContext {
    pub network: ServiceStore<NetworkData>,
    pub network_tx: Sender<NetworkCmd>,

    pub bluetooth: ServiceStore<BluetoothData>,
    pub bluetooth_tx: Sender<BluetoothCmd>,

    pub audio: ServiceStore<AudioData>,
    pub audio_tx: Sender<AudioCmd>,

    pub backlight: ServiceStore<BacklightData>,
    pub backlight_tx: Sender<BacklightCmd>,

    pub nightlight: ServiceStore<NightlightData>,
    pub nightlight_tx: Sender<NightlightCmd>,

    pub launcher: ServiceStore<LauncherData>,
    pub launcher_tx: Sender<LauncherCmd>,

    pub notifications: ServiceStore<NotificationData>,
    pub notifications_tx: Sender<NotificationCmd>,

    pub power: ServiceStore<PowerData>,

    pub niri: ServiceStore<NiriData>,
    pub clock: ServiceStore<DateTime<Local>>,

    pub dnd: ServiceStore<DndData>,
    pub dnd_tx: Sender<DndCmd>,

    pub tray: ServiceStore<TrayData>,
    pub tray_tx: Sender<TrayCmd>,
}

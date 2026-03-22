use chrono::{DateTime, Local};

use crate::services::audio::{AudioCmd, AudioData};
use crate::services::backlight::{BacklightCmd, BacklightData};
use crate::services::bluetooth::{BluetoothCmd, BluetoothData};
use crate::services::dnd::{DndCmd, DndData};
use crate::services::kdeconnect::{KdeConnectCmd, KdeConnectData};
use crate::services::launcher::{LauncherCmd, LauncherData};
use crate::services::network::{NetworkCmd, NetworkData};
use crate::services::nightlight::{NightlightCmd, NightlightData};
use crate::services::niri::NiriData;
use crate::services::notifications::{server::NotificationCmd, NotificationData};
use crate::services::power::PowerData;
use crate::services::tray::{TrayCmd, TrayData};
use crate::store::{ReadOnlyHandle, ServiceHandle};

/// Zentraler App-Kontext — wird an alle Widgets weitergegeben.
///
/// Jedes ServiceHandle koppelt Store + Sender zusammen.
/// Read-only Services nutzen ReadOnlyHandle (kein Sender).
#[derive(Clone)]
pub struct AppContext {
    pub network: ServiceHandle<NetworkData, NetworkCmd>,
    pub bluetooth: ServiceHandle<BluetoothData, BluetoothCmd>,
    pub audio: ServiceHandle<AudioData, AudioCmd>,
    pub backlight: ServiceHandle<BacklightData, BacklightCmd>,
    pub nightlight: ServiceHandle<NightlightData, NightlightCmd>,
    pub launcher: ServiceHandle<LauncherData, LauncherCmd>,
    pub notifications: ServiceHandle<NotificationData, NotificationCmd>,
    pub dnd: ServiceHandle<DndData, DndCmd>,
    pub tray: ServiceHandle<TrayData, TrayCmd>,
    pub kdeconnect: ServiceHandle<KdeConnectData, KdeConnectCmd>,
    pub power: ReadOnlyHandle<PowerData>,
    pub niri: ReadOnlyHandle<NiriData>,
    pub clock: ReadOnlyHandle<DateTime<Local>>,
}

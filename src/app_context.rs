use crate::services::notifications::{server::NotificationCmd, NotificationData};
use chrono::{DateTime, Local};

use crate::services::airplane::{AirplaneCmd, AirplaneData};
use crate::services::audio::{AudioCmd, AudioData};
use crate::services::backlight::{BacklightCmd, BacklightData};
use crate::services::bluetooth::{BluetoothCmd, BluetoothData};
use crate::services::dnd::{DndCmd, DndData};
use crate::services::kdeconnect::{KdeConnectCmd, KdeConnectData};
use crate::services::launcher::{LauncherCmd, LauncherData};
use crate::services::network::{NetworkCmd, NetworkData};
use crate::services::nightlight::{NightlightCmd, NightlightData};
use crate::services::niri::NiriData;
use crate::services::power::PowerData;
use crate::services::tasks::TaskRegistry;
use crate::services::tray::{TrayCmd, TrayData};
use crate::services::Service;
use crate::store::{ReadOnlyHandle, ServiceHandle, ServiceStore};
use std::sync::{Arc, Mutex};

pub fn spawn_service<S: Service>() -> ServiceHandle<S::Data, S::Cmd> {
    let (store, tx) = S::spawn();
    ServiceHandle { store, tx }
}

pub fn spawn_readonly<S: Service<Cmd = ()>>() -> ReadOnlyHandle<S::Data> {
    let (store, _) = S::spawn();
    ReadOnlyHandle { store }
}

#[derive(Clone)]
pub struct AppContext {
    pub airplane: ServiceHandle<AirplaneData, AirplaneCmd>,
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
    pub task_registry: Arc<Mutex<TaskRegistry>>,
}

use axis_core::services::notifications::NotificationData;
use crate::services::notifications::server::NotificationCmd;
use chrono::{DateTime, Local};

use axis_core::services::airplane::{AirplaneCmd, AirplaneData};
use axis_core::services::audio::{AudioCmd, AudioData};
use axis_core::services::backlight::{BacklightCmd, BacklightData};
use axis_core::services::bluetooth::{BluetoothCmd, BluetoothData};
use axis_core::services::calendar::CalendarRegistry;
use axis_core::services::continuity::{ContinuityCmd, ContinuityData};
use axis_core::services::dnd::{DndCmd, DndData};
use axis_core::services::settings::{SettingsCmd, SettingsData};
use axis_core::services::kdeconnect::{KdeConnectCmd, KdeConnectData};
use crate::services::launcher::{LauncherCmd, LauncherData};
use axis_core::services::network::{NetworkCmd, NetworkData};
use axis_core::services::nightlight::{NightlightCmd, NightlightData};
use crate::services::niri::NiriData;
use axis_core::services::power::PowerData;
use axis_core::services::tasks::TaskRegistry;
use axis_core::services::tray::{TrayCmd, TrayData};
use axis_core::{Service, ReadOnlyHandle, ServiceHandle};
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
    pub continuity: ServiceHandle<ContinuityData, ContinuityCmd>,
    pub settings: ServiceHandle<SettingsData, SettingsCmd>,
    pub calendar_registry: Arc<Mutex<CalendarRegistry>>,
}

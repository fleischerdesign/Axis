use async_channel::Sender;
use chrono::{DateTime, Local};
use std::cell::RefCell;
use std::rc::Rc;

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
use crate::services::notifications::{server::NotificationCmd, Notification, NotificationData};
use crate::services::power::PowerData;
use crate::services::tasks::TaskRegistry;
use crate::services::tray::{TrayCmd, TrayData};
use crate::store::{ReadOnlyHandle, ServiceHandle};

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
    pub notification_raw_tx: Sender<Notification>,
    pub dnd: ServiceHandle<DndData, DndCmd>,
    pub tray: ServiceHandle<TrayData, TrayCmd>,
    pub kdeconnect: ServiceHandle<KdeConnectData, KdeConnectCmd>,
    pub power: ReadOnlyHandle<PowerData>,
    pub niri: ReadOnlyHandle<NiriData>,
    pub clock: ReadOnlyHandle<DateTime<Local>>,
    pub task_registry: Rc<RefCell<TaskRegistry>>,
}

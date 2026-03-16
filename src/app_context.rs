use async_channel::Sender;
use chrono::{DateTime, Local};

use crate::services::audio::{AudioCmd, AudioData};
use crate::services::backlight::{BacklightCmd, BacklightData};
use crate::services::bluetooth::{BluetoothCmd, BluetoothData};
use crate::services::network::{NetworkCmd, NetworkData};
use crate::services::nightlight::{NightlightCmd, NightlightData};
use crate::services::niri::NiriData;
use crate::services::power::PowerData;
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

    pub power: ServiceStore<PowerData>,

    pub niri: ServiceStore<NiriData>,
    pub clock: ServiceStore<DateTime<Local>>,
}

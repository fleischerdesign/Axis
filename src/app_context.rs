use async_channel::{Receiver, Sender};
use chrono::{DateTime, Local};

use crate::services::audio::{AudioCmd, AudioData};
use crate::services::bluetooth::{BluetoothCmd, BluetoothData};
use crate::services::network::{NetworkCmd, NetworkData};
use crate::services::niri::NiriData;
use crate::services::power::PowerData;

#[derive(Clone)]
pub struct AppContext {
    pub network_rx: Receiver<NetworkData>,
    pub network_tx: Sender<NetworkCmd>,

    pub bluetooth_rx: Receiver<BluetoothData>,
    pub bluetooth_tx: Sender<BluetoothCmd>,

    pub audio_rx: Receiver<AudioData>,
    pub audio_tx: Sender<AudioCmd>,

    pub power_rx: Receiver<PowerData>,

    pub niri_rx: Receiver<NiriData>,
    pub clock_rx: Receiver<DateTime<Local>>,
}

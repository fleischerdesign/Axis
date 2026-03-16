use tokio::sync::watch;
use futures_channel::mpsc;
use chrono::{Local, DateTime};

use crate::services::network::{NetworkData, NetworkCmd};
use crate::services::bluetooth::{BluetoothData, BluetoothCmd};
use crate::services::audio::{AudioData, AudioCmd};
use crate::services::power::PowerData;
use crate::services::niri::NiriData;

#[derive(Clone)]
pub struct AppContext {
    pub network_rx: watch::Receiver<NetworkData>,
    pub network_tx: mpsc::UnboundedSender<NetworkCmd>,

    pub bluetooth_rx: watch::Receiver<BluetoothData>,
    pub bluetooth_tx: mpsc::UnboundedSender<BluetoothCmd>,

    pub audio_rx: watch::Receiver<AudioData>,
    pub audio_tx: mpsc::UnboundedSender<AudioCmd>,

    pub power_rx: watch::Receiver<PowerData>,

    pub niri_rx: watch::Receiver<NiriData>,
    pub clock_rx: watch::Receiver<DateTime<Local>>,
}

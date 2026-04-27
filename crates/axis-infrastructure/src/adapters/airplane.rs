use axis_domain::models::airplane::AirplaneStatus;
use axis_domain::ports::airplane::{AirplaneError, AirplaneProvider, AirplaneStream};
use axis_domain::ports::config::ConfigProvider;
use async_trait::async_trait;
use log::{error, info, warn};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::WatchStream;

const RFKILL_OP_ADD: u8 = 0;
const RFKILL_OP_DEL: u8 = 1;
const RFKILL_OP_CHANGE: u8 = 2;
const RFKILL_OP_CHANGE_ALL: u8 = 3;

const RFKILL_TYPE_WLAN: u8 = 1;
const RFKILL_TYPE_BLUETOOTH: u8 = 2;
const RFKILL_TYPE_WWAN: u8 = 5;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RfkillEvent {
    idx: u32,
    type_: u8,
    op: u8,
    soft: u8,
    hard: u8,
}

struct DeviceMap {
    devices: HashMap<u32, (u8, u8, u8)>,
}

impl DeviceMap {
    fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }
}

fn is_wireless(t: u8) -> bool {
    matches!(t, RFKILL_TYPE_WLAN | RFKILL_TYPE_WWAN | RFKILL_TYPE_BLUETOOTH)
}

fn compute_airplane(map: &DeviceMap) -> bool {
    let wireless: Vec<_> = map
        .devices
        .values()
        .filter(|(t, _, _)| *t == RFKILL_TYPE_WLAN || *t == RFKILL_TYPE_WWAN)
        .collect();

    if wireless.is_empty() {
        return false;
    }

    wireless.iter().all(|(_, soft, _)| *soft != 0)
}

fn write_rfkill_change_all(rfkill_type: u8, blocked: bool) -> std::io::Result<()> {
    let event = RfkillEvent {
        idx: 0,
        type_: rfkill_type,
        op: RFKILL_OP_CHANGE_ALL,
        soft: if blocked { 1 } else { 0 },
        hard: 0,
    };

    // SAFETY: RfkillEvent is #[repr(C, packed)] with only integer fields,
    // so it has no padding bytes and no invalid bit patterns. The byte slice
    // is created from a valid reference with the correct size.
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &event as *const _ as *const u8,
            std::mem::size_of::<RfkillEvent>(),
        )
    };

    let mut f = std::fs::File::options().write(true).open("/dev/rfkill")?;
    f.write_all(bytes)?;

    info!(
        "[airplane] rfkill {} {:?}",
        if blocked { "blocked" } else { "unblocked" },
        match rfkill_type {
            RFKILL_TYPE_WLAN => "WLAN",
            RFKILL_TYPE_BLUETOOTH => "BLUETOOTH",
            RFKILL_TYPE_WWAN => "WWAN",
            _ => "unknown",
        }
    );

    Ok(())
}

enum AirplaneCmd {
    Sync(bool),
}

pub struct ConfigAirplaneProvider {
    config_provider: Arc<dyn ConfigProvider>,
    status_tx: watch::Sender<AirplaneStatus>,
    cmd_tx: mpsc::Sender<AirplaneCmd>,
}

impl ConfigAirplaneProvider {
    pub async fn new(config_provider: Arc<dyn ConfigProvider>) -> Arc<Self> {
        let available = std::fs::File::open("/dev/rfkill").is_ok();
        let initial_enabled = config_provider.get().expect("config get failed").airplane.enabled;

        let initial_status = AirplaneStatus {
            enabled: initial_enabled,
            available,
        };

        let (status_tx, _) = watch::channel(initial_status);
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<AirplaneCmd>(32);

        let state = Arc::new(Mutex::new(DeviceMap::new()));

        if available && initial_enabled {
            let _ = cmd_tx.try_send(AirplaneCmd::Sync(true));
        }

        let state_reader = state.clone();
        let status_tx_reader = status_tx.clone();
        std::thread::spawn(move || {
            let mut rfkill_fd = match std::fs::File::open("/dev/rfkill") {
                Ok(f) => f,
                Err(e) => {
                    error!("[airplane] Cannot open /dev/rfkill: {e}");
                    return;
                }
            };

            let mut buf = [0u8; std::mem::size_of::<RfkillEvent>()];

            loop {
                match rfkill_fd.read_exact(&mut buf) {
                    Ok(()) => {
                        // SAFETY: RfkillEvent is #[repr(C, packed)] with only integer
                        // fields and no invalid bit patterns. buf was filled by
                        // read_exact with exactly size_of::<RfkillEvent>() bytes.
                        let event: RfkillEvent =
                            unsafe { std::ptr::read(buf.as_ptr() as *const _) };

                        if !is_wireless(event.type_) {
                            continue;
                        }

                        let idx = event.idx;

                        let mut map = state_reader.lock().unwrap();
                        match event.op {
                            RFKILL_OP_ADD | RFKILL_OP_CHANGE => {
                                map.devices.insert(idx, (event.type_, event.soft, event.hard));
                            }
                            RFKILL_OP_DEL => {
                                map.devices.remove(&idx);
                            }
                            _ => continue,
                        }

                        let enabled = compute_airplane(&map);
                        drop(map);

                        status_tx_reader.send_modify(|s| s.enabled = enabled);
                    }
                    Err(e) => {
                        warn!("[airplane] /dev/rfkill read error: {e}");
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        });


        let status_tx_cmd = status_tx.clone();
        std::thread::spawn(move || {
            while let Some(cmd) = cmd_rx.blocking_recv() {
                match cmd {
                    AirplaneCmd::Sync(on) => {
                        if let Err(e) = write_rfkill_change_all(RFKILL_TYPE_WLAN, on) {
                            warn!("[airplane] Failed to toggle WLAN: {e}");
                        }
                        if let Err(e) = write_rfkill_change_all(RFKILL_TYPE_BLUETOOTH, on) {
                            warn!("[airplane] Failed to toggle Bluetooth: {e}");
                        }
                        if let Err(e) = write_rfkill_change_all(RFKILL_TYPE_WWAN, on) {
                            warn!("[airplane] Failed to toggle WWAN: {e}");
                        }

                        status_tx_cmd.send_modify(|s| s.enabled = on);
                    }
                }
            }
        });

        let provider = Arc::new(Self {
            config_provider: config_provider.clone(),
            status_tx,
            cmd_tx,
        });

        let cmd_tx_bg = provider.cmd_tx.clone();
        let mut last_enabled = initial_enabled;
        tokio::spawn(async move {
            let mut stream = config_provider.subscribe().expect("config subscribe failed");
            while let Some(config) = futures_util::StreamExt::next(&mut stream).await {
                let desired = config.airplane.enabled;
                if desired != last_enabled {
                    last_enabled = desired;
                    let _ = cmd_tx_bg.send(AirplaneCmd::Sync(desired)).await;
                }
            }
        });

        provider
    }
}

#[async_trait]
impl AirplaneProvider for ConfigAirplaneProvider {
    async fn get_status(&self) -> Result<AirplaneStatus, AirplaneError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<AirplaneStream, AirplaneError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), AirplaneError> {
        self.config_provider
            .update(Box::new(move |cfg| cfg.airplane.enabled = enabled))
            .map_err(|e| AirplaneError::ProviderError(e.to_string()))
    }
}

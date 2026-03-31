use super::{Service, ServiceConfig};
use crate::store::ServiceStore;
use async_channel::{bounded, Sender};
use log::{error, info, warn};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};

// ── Public API ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AirplaneData {
    pub enabled: bool,
}

pub enum AirplaneCmd {
    Toggle(bool),
}

pub struct AirplaneService;

impl Service for AirplaneService {
    type Data = AirplaneData;
    type Cmd = AirplaneCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(16);
        let (cmd_tx, cmd_rx) = bounded(16);

        let state = Arc::new(Mutex::new(DeviceMap::new()));

        // ── Reader thread: blocks on /dev/rfkill ────────────────────
        let reader_state = state.clone();
        let reader_tx = data_tx.clone();
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
                        let event: RfkillEvent =
                            unsafe { std::ptr::read(buf.as_ptr() as *const _) };

                        if !is_wireless(event.type_) {
                            continue;
                        }

                        let idx = event.idx;

                        let mut map = reader_state.lock().unwrap();
                        match event.op {
                            RFKILL_OP_ADD | RFKILL_OP_CHANGE => {
                                map.devices
                                    .insert(idx, (event.soft, event.hard));
                            }
                            RFKILL_OP_DEL => {
                                map.devices.remove(&idx);
                            }
                            _ => continue,
                        }

                        let enabled = compute_airplane(&map);
                        drop(map);

                        let _ = reader_tx.send_blocking(AirplaneData { enabled });
                    }
                    Err(e) => {
                        warn!("[airplane] /dev/rfkill read error: {e}");
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        });

        // ── Command thread: handles Toggle ──────────────────────────
        let _handler_state = state;
        let handler_tx = data_tx;
        std::thread::spawn(move || {
            loop {
                match cmd_rx.recv_blocking() {
                    Ok(AirplaneCmd::Toggle(on)) => {
                        if let Err(e) = write_rfkill_change_all(RFKILL_TYPE_WLAN, on) {
                            warn!("[airplane] Failed to toggle WLAN: {e}");
                        }
                        if let Err(e) = write_rfkill_change_all(RFKILL_TYPE_WWAN, on) {
                            warn!("[airplane] Failed to toggle WWAN: {e}");
                        }

                        // After writing, the reader thread will get CHANGE events
                        // and recompute + publish. But in case the reader hasn't
                        // processed yet, publish our intended state immediately.
                        let _ = handler_tx.send_blocking(AirplaneData { enabled: on });
                    }
                    Err(_) => break,
                }
            }
        });

        (ServiceStore::new(data_rx, AirplaneData::default()), cmd_tx)
    }
}

impl ServiceConfig for AirplaneService {
    fn get_enabled(data: &AirplaneData) -> bool { data.enabled }
    fn cmd_set_enabled(on: bool) -> AirplaneCmd { AirplaneCmd::Toggle(on) }
}

// ── rfkill Kernel Interface ───────────────────────────────────────────

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RfkillEvent {
    idx: u32,
    type_: u8,
    op: u8,
    soft: u8,
    hard: u8,
}

const RFKILL_OP_ADD: u8 = 0;
const RFKILL_OP_DEL: u8 = 1;
const RFKILL_OP_CHANGE: u8 = 2;
const RFKILL_OP_CHANGE_ALL: u8 = 3;

const RFKILL_TYPE_WLAN: u8 = 1;
const RFKILL_TYPE_BLUETOOTH: u8 = 2;
const RFKILL_TYPE_WWAN: u8 = 5;

// ── Device State Tracking ────────────────────────────────────────────

struct DeviceMap {
    devices: HashMap<u32, (u8, u8)>, // idx → (soft, hard)
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
    // Airplane ON when ALL wireless (wlan + wwan) devices are soft-blocked
    let wireless: Vec<_> = map
        .devices
        .values()
        .filter(|(_, t)| *t == RFKILL_TYPE_WLAN || *t == RFKILL_TYPE_WWAN)
        .collect();

    if wireless.is_empty() {
        return false;
    }

    wireless.iter().all(|(soft, _)| *soft != 0)
}

// ── Write to /dev/rfkill ──────────────────────────────────────────────

fn write_rfkill_change_all(rfkill_type: u8, blocked: bool) -> std::io::Result<()> {
    let event = RfkillEvent {
        idx: 0,
        type_: rfkill_type,
        op: RFKILL_OP_CHANGE_ALL,
        soft: if blocked { 1 } else { 0 },
        hard: 0,
    };

    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            &event as *const _ as *const u8,
            std::mem::size_of::<RfkillEvent>(),
        )
    };

    use std::io::Write;
    let mut f = std::fs::File::options().write(true).open("/dev/rfkill")?;
    f.write_all(bytes)?;

    info!(
        "[airplane] rfkill {} {:?}",
        if blocked { "blocked" } else { "unblocked" },
        match rfkill_type {
            RFKILL_TYPE_WLAN => "WLAN",
            RFKILL_TYPE_WWAN => "WWAN",
            _ => "unknown",
        }
    );

    Ok(())
}

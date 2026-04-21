use super::{Service, ServiceConfig};
use crate::store::ServiceStore;
use async_channel::{bounded, Sender};
use log::info;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DndData {
    pub enabled: bool,
}

pub enum DndCmd {
    Toggle(bool),
}

pub struct DndService;

impl Service for DndService {
    type Data = DndData;
    type Cmd = DndCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(16);
        let (cmd_tx, cmd_rx) = bounded(16);

        std::thread::spawn(move || {
            let mut data = DndData::default();

            loop {
                match cmd_rx.recv_blocking() {
                    Ok(DndCmd::Toggle(on)) => {
                        info!("[dnd] {}", if on { "enabled" } else { "disabled" });
                        data.enabled = on;
                        let _ = data_tx.send_blocking(data.clone());
                    }
                    Err(_) => break,
                }
            }
        });

        (ServiceStore::new(data_rx, DndData::default()), cmd_tx)
    }
}

impl ServiceConfig for DndService {
    fn get_enabled(data: &DndData) -> bool { data.enabled }
    fn cmd_set_enabled(on: bool) -> DndCmd { DndCmd::Toggle(on) }
}

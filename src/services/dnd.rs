use super::Service;
use crate::store::ServiceStore;
use async_channel::{bounded, Sender};

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
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        DndCmd::Toggle(on) => {
                            data.enabled = on;
                            let _ = data_tx.send_blocking(data.clone());
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        (ServiceStore::new(data_rx, DndData::default()), cmd_tx)
    }
}

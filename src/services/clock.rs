use async_channel::{bounded, Sender};
use chrono::{DateTime, Local};
use std::time::Duration;

use super::Service;
use crate::store::ServiceStore;

pub struct ClockService;

impl Service for ClockService {
    type Data = DateTime<Local>;
    type Cmd = ();

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (tx, rx) = bounded(10);

        tokio::spawn(async move {
            loop {
                if tx.send(Local::now()).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        let (dummy_tx, _) = bounded(1);
        (ServiceStore::new(rx, Local::now()), dummy_tx)
    }
}

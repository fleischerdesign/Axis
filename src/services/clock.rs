use async_channel::{bounded, Receiver, Sender};
use chrono::{DateTime, Local};
use std::thread;
use std::time::Duration;

use super::traits::Service;

pub struct ClockService;

impl Service for ClockService {
    type Data = DateTime<Local>;
    type Cmd = ();

    fn spawn() -> (Receiver<Self::Data>, Sender<Self::Cmd>) {
        let (tx, rx) = bounded(10);

        thread::spawn(move || loop {
            if tx.send_blocking(Local::now()).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(1000));
        });

        let (dummy_tx, _) = bounded(1);
        (rx, dummy_tx)
    }
}

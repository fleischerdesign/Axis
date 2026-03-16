use async_channel::{bounded, Receiver, Sender};
use chrono::{DateTime, Local};
use std::thread;
use std::time::Duration;

pub struct ClockService;

impl ClockService {
    pub fn spawn() -> (Receiver<DateTime<Local>>, Sender<DateTime<Local>>) {
        let (data_tx, data_rx) = bounded(100);
        let data_tx_return = data_tx.clone();

        thread::spawn(move || loop {
            let _ = data_tx.send_blocking(Local::now());
            thread::sleep(Duration::from_millis(1000));
        });

        (data_rx, data_tx_return)
    }
}

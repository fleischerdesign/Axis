use async_channel::{bounded, Receiver};
use chrono::{DateTime, Local};
use std::thread;
use std::time::Duration;

pub struct ClockService;

impl ClockService {
    pub fn spawn() -> Receiver<DateTime<Local>> {
        let (tx, rx) = bounded(10);

        thread::spawn(move || loop {
            let _ = tx.send_blocking(Local::now());
            thread::sleep(Duration::from_millis(1000));
        });

        rx
    }
}

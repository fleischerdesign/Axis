use std::thread;
use std::time::Duration;
use chrono::{Local, DateTime};

pub struct ClockService;

impl ClockService {
    pub fn spawn() -> tokio::sync::watch::Receiver<DateTime<Local>> {
        let (tx, rx) = tokio::sync::watch::channel(Local::now());

        thread::spawn(move || {
            loop {
                let _ = tx.send(Local::now());
                thread::sleep(Duration::from_millis(1000));
            }
        });

        rx
    }
}

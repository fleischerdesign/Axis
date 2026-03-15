use std::process::Command;
use std::thread;
use std::time::Duration;
use futures_channel::mpsc;
use futures_util::StreamExt;

#[derive(Clone, Debug)]
pub struct AudioData {
    pub volume: f64, // 0.0 bis 1.0
    pub is_muted: bool,
}

pub struct AudioService;

impl AudioService {
    pub fn spawn() -> (mpsc::UnboundedReceiver<AudioData>, mpsc::UnboundedSender<f64>) {
        let (data_tx, data_rx) = mpsc::unbounded();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded::<f64>();

        // Thread 1: Daten abfragen (Polling wpctl)
        let tx_clone = data_tx.clone();
        thread::spawn(move || {
            loop {
                if let Ok(data) = Self::get_wpctl_status() {
                    if tx_clone.unbounded_send(data).is_err() { break; }
                }
                thread::sleep(Duration::from_millis(500));
            }
        });

        // Thread 2: Lautstärke setzen
        thread::spawn(move || {
            while let Some(new_vol) = futures_executor::block_on(cmd_rx.next()) {
                let _ = Command::new("wpctl")
                    .arg("set-volume")
                    .arg("@DEFAULT_AUDIO_SINK@")
                    .arg(format!("{}%", (new_vol * 100.0) as i32))
                    .output();
            }
        });

        (data_rx, cmd_tx)
    }

    fn get_wpctl_status() -> Result<AudioData, String> {
        let output = Command::new("wpctl")
            .arg("get-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .output()
            .map_err(|e| e.to_string())?;

        let s = String::from_utf8_lossy(&output.stdout);
        // Beispiel-Output: "Volume: 0.45" oder "Volume: 0.45 [MUTED]"
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() >= 2 {
            let vol = parts[1].parse::<f64>().unwrap_or(0.0);
            let muted = s.contains("[MUTED]");
            Ok(AudioData { volume: vol, is_muted: muted })
        } else {
            Err("Invalid wpctl output".into())
        }
    }
}

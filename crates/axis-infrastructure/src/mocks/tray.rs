use axis_domain::models::tray::{TrayItem, TrayItemStatus, TrayStatus};
use axis_domain::ports::tray::{TrayError, TrayProvider, TrayStream};
use async_trait::async_trait;
use log::info;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct MockTrayProvider {
    status_tx: watch::Sender<TrayStatus>,
}

impl MockTrayProvider {
    pub fn new() -> Self {
        let (tx, _) = watch::channel(TrayStatus {
            items: vec![
                TrayItem {
                    bus_name: "org.freedesktop.StatusNotifierItem-mock-1".to_string(),
                    id: "nm-applet".to_string(),
                    title: "Network".to_string(),
                    icon_name: "network-wireless-symbolic".to_string(),
                    attention_icon_name: String::new(),
                    overlay_icon_name: String::new(),
                    icon_pixmap: vec![],
                    attention_icon_pixmap: vec![],
                    status: TrayItemStatus::Active,
                },
                TrayItem {
                    bus_name: "org.freedesktop.StatusNotifierItem-mock-2".to_string(),
                    id: "bluetooth".to_string(),
                    title: "Bluetooth".to_string(),
                    icon_name: "bluetooth-symbolic".to_string(),
                    attention_icon_name: String::new(),
                    overlay_icon_name: String::new(),
                    icon_pixmap: vec![],
                    attention_icon_pixmap: vec![],
                    status: TrayItemStatus::Active,
                },
            ],
        });
        Self { status_tx: tx }
    }
}

#[async_trait]
impl TrayProvider for MockTrayProvider {
    async fn get_status(&self) -> Result<TrayStatus, TrayError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<TrayStream, TrayError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn activate(&self, bus_name: &str, _x: i32, _y: i32) -> Result<(), TrayError> {
        info!("[tray-mock] activate: {bus_name}");
        Ok(())
    }

    async fn context_menu(&self, bus_name: &str, _x: i32, _y: i32) -> Result<(), TrayError> {
        info!("[tray-mock] context_menu: {bus_name}");
        Ok(())
    }

    async fn secondary_activate(&self, bus_name: &str, _x: i32, _y: i32) -> Result<(), TrayError> {
        info!("[tray-mock] secondary_activate: {bus_name}");
        Ok(())
    }

    async fn scroll(&self, bus_name: &str, _delta: i32, _orientation: &str) -> Result<(), TrayError> {
        info!("[tray-mock] scroll: {bus_name}");
        Ok(())
    }
}

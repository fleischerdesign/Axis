use futures_util::StreamExt;
use zbus::{proxy, Connection};
use futures_channel::mpsc;

#[proxy(
    interface = "org.bluez.Adapter1",
    default_service = "org.bluez",
    default_path = "/org/bluez/hci0"
)]
trait BluetoothAdapter {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
}

#[derive(Debug, Clone, Default)]
pub struct BluetoothData {
    pub is_powered: bool,
}

pub struct BluetoothService;

impl BluetoothService {
    pub fn spawn() -> (mpsc::UnboundedReceiver<BluetoothData>, mpsc::UnboundedSender<bool>) {
        let (data_tx, data_rx) = mpsc::unbounded();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded::<bool>();

        tokio::spawn(async move {
            let connection = Connection::system().await.unwrap();
            // Wir gehen davon aus, dass hci0 der Standard-Adapter ist
            let proxy = BluetoothAdapterProxy::new(&connection).await.unwrap();

            let mut powered_changed = proxy.receive_powered_changed().await;

            // Initialer Fetch
            let powered = proxy.powered().await.unwrap_or(false);
            let _ = data_tx.unbounded_send(BluetoothData { is_powered: powered });

            loop {
                tokio::select! {
                    // Auf Änderungen von BlueZ reagieren
                    Some(_) = powered_changed.next() => {
                        let powered = proxy.powered().await.unwrap_or(false);
                        let _ = data_tx.unbounded_send(BluetoothData { is_powered: powered });
                    }
                    // Auf Befehle von der UI reagieren
                    Some(on) = cmd_rx.next() => {
                        let _ = proxy.set_powered(on).await;
                    }
                }
            }
        });

        (data_rx, cmd_tx)
    }
}

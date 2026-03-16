use async_channel::Receiver;
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;

/// Ein reaktiver Store für einen Service-Datentyp.
///
/// Läuft vollständig auf dem GTK-Main-Thread (via `glib::spawn_future_local`).
/// Kein Tokio, kein Blocking. Empfängt Daten aus einem `async_channel::Receiver`
/// und benachrichtigt alle registrierten Subscriber synchron.
///
/// # Nutzung
/// ```rust
/// let store = ServiceStore::new(rx, PowerData::default());
/// store.subscribe(|data| label.set_text(&format!("{}%", data.battery_percentage)));
/// // Letzten bekannten Wert sofort lesen:
/// let current = store.get();
/// ```
pub struct ServiceStore<T: Clone + 'static> {
    data: Rc<RefCell<T>>,
    listeners: Rc<RefCell<Vec<Box<dyn Fn(&T)>>>>,
}

impl<T: Clone + 'static> ServiceStore<T> {
    /// Erstellt einen neuen Store und startet den GLib-Empfangsloop.
    pub fn new(rx: Receiver<T>, initial: T) -> Self {
        let data = Rc::new(RefCell::new(initial));
        let listeners: Rc<RefCell<Vec<Box<dyn Fn(&T)>>>> = Rc::new(RefCell::new(Vec::new()));

        let data_c = data.clone();
        let listeners_c = listeners.clone();

        glib::spawn_future_local(async move {
            while let Ok(new_data) = rx.recv().await {
                *data_c.borrow_mut() = new_data.clone();
                for listener in listeners_c.borrow().iter() {
                    listener(&new_data);
                }
            }
        });

        Self { data, listeners }
    }

    /// Registriert einen Callback, der bei jedem neuen Datenwert aufgerufen wird.
    /// Der Callback wird sofort mit dem aktuellen Wert aufgerufen.
    pub fn subscribe(&self, f: impl Fn(&T) + 'static) {
        // Sofort mit aktuellem Stand aufrufen (kein "warten auf erstes Event")
        f(&self.data.borrow());
        self.listeners.borrow_mut().push(Box::new(f));
    }

    /// Gibt eine Kopie des letzten bekannten Wertes zurück.
    pub fn get(&self) -> T {
        self.data.borrow().clone()
    }
}

impl<T: Clone + 'static> Clone for ServiceStore<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            listeners: self.listeners.clone(),
        }
    }
}

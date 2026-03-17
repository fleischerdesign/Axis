use async_channel::Receiver;
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;

/// Ein reaktiver Store für einen Service-Datentyp.
///
/// Läuft vollständig auf dem GTK-Main-Thread (via `glib::spawn_future_local`).
/// Kein Tokio, kein Blocking. Empfängt Daten aus einem `async_channel::Receiver`
/// und benachrichtigt alle registrierten Subscriber synchron.
pub struct ServiceStore<T: Clone + 'static> {
    pub store: Store<T>,
}

impl<T: Clone + 'static> ServiceStore<T> {
    /// Erstellt einen neuen Store und startet den GLib-Empfangsloop.
    pub fn new(rx: Receiver<T>, initial: T) -> Self {
        let store = Store::new(initial);
        let store_c = store.clone();

        glib::spawn_future_local(async move {
            while let Ok(new_data) = rx.recv().await {
                store_c.set(new_data);
            }
        });

        Self { store }
    }

    /// Erstellt einen Store, der manuell geupdatet werden kann (z.B. für UI-State).
    pub fn new_manual(initial: T) -> Self {
        Self {
            store: Store::new(initial),
        }
    }

    pub fn subscribe(&self, f: impl Fn(&T) + 'static) {
        self.store.subscribe(f);
    }

    pub fn get(&self) -> T {
        self.store.get()
    }
}

impl<T: Clone + 'static> Clone for ServiceStore<T> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
        }
    }
}

/// Ein einfacher, thread-lokaler (GTK-Main-Thread) reaktiver Datenspeicher.
pub struct Store<T: Clone + 'static> {
    data: Rc<RefCell<T>>,
    listeners: Rc<RefCell<Vec<Box<dyn Fn(&T)>>>>,
}

impl<T: Clone + 'static> Store<T> {
    pub fn new(initial: T) -> Self {
        Self {
            data: Rc::new(RefCell::new(initial)),
            listeners: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn subscribe(&self, f: impl Fn(&T) + 'static) {
        f(&self.data.borrow());
        self.listeners.borrow_mut().push(Box::new(f));
    }

    pub fn get(&self) -> T {
        self.data.borrow().clone()
    }

    pub fn set(&self, val: T) {
        *self.data.borrow_mut() = val.clone();
        for listener in self.listeners.borrow().iter() {
            listener(&val);
        }
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut data = self.data.borrow_mut();
        f(&mut *data);
        let val = data.clone();
        for listener in self.listeners.borrow().iter() {
            listener(&val);
        }
    }
}

impl<T: Clone + 'static> Clone for Store<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            listeners: self.listeners.clone(),
        }
    }
}

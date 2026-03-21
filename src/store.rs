use async_channel::{Receiver, Sender};
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;

/// Pairs a reactive store with a command sender for a single service.
pub struct ServiceHandle<T: Clone + PartialEq + 'static, C: Send + 'static> {
    pub store: ServiceStore<T>,
    pub tx: Sender<C>,
}

impl<T: Clone + PartialEq + 'static, C: Send + 'static> Clone for ServiceHandle<T, C> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            tx: self.tx.clone(),
        }
    }
}

impl<T: Clone + PartialEq + 'static, C: Send + 'static> ServiceHandle<T, C> {
    pub fn new(rx: Receiver<T>, initial: T, tx: Sender<C>) -> Self {
        Self {
            store: ServiceStore::new(rx, initial),
            tx,
        }
    }

    pub fn subscribe(&self, f: impl Fn(&T) + 'static) {
        self.store.subscribe(f);
    }

    pub fn get(&self) -> T {
        self.store.get()
    }
}

/// Same as ServiceHandle but for read-only services (no command sender).
#[derive(Clone)]
pub struct ReadOnlyHandle<T: Clone + PartialEq + 'static> {
    pub store: ServiceStore<T>,
}

impl<T: Clone + PartialEq + 'static> ReadOnlyHandle<T> {
    pub fn new(rx: Receiver<T>, initial: T) -> Self {
        Self {
            store: ServiceStore::new(rx, initial),
        }
    }

    pub fn subscribe(&self, f: impl Fn(&T) + 'static) {
        self.store.subscribe(f);
    }

    pub fn get(&self) -> T {
        self.store.get()
    }
}

/// Ein reaktiver Store für einen Service-Datentyp.
pub struct ServiceStore<T: Clone + PartialEq + 'static> {
    pub store: Store<T>,
}

impl<T: Clone + PartialEq + 'static> ServiceStore<T> {
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

impl<T: Clone + PartialEq + 'static> Clone for ServiceStore<T> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
        }
    }
}

/// Ein einfacher, thread-lokaler reaktiver Datenspeicher.
pub struct Store<T: Clone + PartialEq + 'static> {
    data: Rc<RefCell<T>>,
    listeners: Rc<RefCell<Vec<Box<dyn Fn(&T)>>>>,
}

impl<T: Clone + PartialEq + 'static> Store<T> {
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
        let mut data = self.data.borrow_mut();
        if *data != val {
            *data = val.clone();
            drop(data); // Lock lösen vor Callbacks
            for listener in self.listeners.borrow().iter() {
                listener(&val);
            }
        }
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut data = self.data.borrow_mut();
        let old_data = data.clone();
        f(&mut *data);
        if old_data != *data {
            let val = data.clone();
            drop(data); // Lock lösen vor Callbacks
            for listener in self.listeners.borrow().iter() {
                listener(&val);
            }
        }
    }
}

impl<T: Clone + PartialEq + 'static> Clone for Store<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            listeners: self.listeners.clone(),
        }
    }
}

/// A lightweight reactive boolean that notifies listeners on change.
/// Replaces scattered `Rc<RefCell<bool>>` patterns with a unified API.
#[derive(Clone, Default)]
pub struct ReactiveBool {
    inner: Rc<RefCell<bool>>,
    listeners: Rc<RefCell<Vec<Box<dyn Fn(bool)>>>>,
}

impl ReactiveBool {
    pub fn new(initial: bool) -> Self {
        Self {
            inner: Rc::new(RefCell::new(initial)),
            listeners: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn get(&self) -> bool {
        *self.inner.borrow()
    }

    pub fn set(&self, val: bool) {
        let mut data = self.inner.borrow_mut();
        if *data != val {
            *data = val;
            drop(data);
            for listener in self.listeners.borrow().iter() {
                listener(val);
            }
        }
    }

    pub fn toggle(&self) {
        let current = self.get();
        self.set(!current);
    }

    pub fn subscribe(&self, f: impl Fn(bool) + 'static) {
        f(self.get());
        self.listeners.borrow_mut().push(Box::new(f));
    }
}

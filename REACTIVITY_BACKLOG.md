# Axis Reactivity System — Improvement Backlog

## Current State

The reactive store system (`src/store.rs`, 209 lines) serves 35+ subscribers well.
Solid foundation: `std::mem::take` pattern, PartialEq guards, clean async→GTK bridge.

## Identified Improvements (Do NOT implement before SettingsSync is done)

### 1. Consolidate ReactiveBool → Store<bool> (DRY)

**Effort:** Low
**Priority:** Medium

`ReactiveBool` is structurally identical to `Store<bool>` with minor differences:
- Passes `bool` by value (not `&bool`) — trivial
- Has `toggle()` method — 3 lines to add to Store
- Derives `Default` — `Store<bool>` could too

16 usages across 4 files:
- `src/widgets/base/mod.rs` — PopupBase.is_open
- `src/widgets/bar/mod.rs` — popup_open, is_visible, is_hovered
- `src/shell/mod.rs` — bar_popup_state

**Fix:**
```rust
impl Store<bool> {
    pub fn toggle(&self) {
        let current = self.get();
        self.set(!current);
    }
}
```
Then replace all `ReactiveBool` usages with `Store<bool>` and delete `ReactiveBool`.

---

### 2. Add Unsubscribe via SubscriptionGuard (Future-proofing)

**Effort:** Medium
**Priority:** Low (only needed if transient subscriptions are required)

Currently all 35 subscriptions are process-lifetime. No unsubscribe mechanism exists.
Fine for now — becomes a problem if popups need to subscribe/unsubscribe on open/close.

**Fix:**
```rust
pub fn subscribe(&self, f: impl Fn(&T) + 'static) -> SubscriptionGuard {
    f(&self.data.borrow());
    let id = self.next_id.get();
    self.next_id.set(id + 1);
    self.listeners.borrow_mut().push((id, Box::new(f)));
    SubscriptionGuard { store: self.clone(), id }
}

pub struct SubscriptionGuard<T: Clone + PartialEq + 'static> {
    store: Store<T>,
    id: usize,
}
impl<T: Clone + PartialEq + 'static> Drop for SubscriptionGuard<T> {
    fn drop(&mut self) {
        self.store.listeners.borrow_mut().retain(|(i, _)| *i != self.id);
    }
}
```

Internal listeners type changes from `Vec<Box<dyn Fn(&T)>>` to `Vec<(usize, Box<dyn Fn(&T)>)>`.

---

### 3. Derived/Computed Stores (Multi-Store Reactivity)

**Effort:** High
**Priority:** Low (only 1 use case currently, handled imperatively)

Currently no way to reactively combine two stores. Only case:
`src/widgets/bar/mod.rs:135` — `popup_open.get() || is_hovered.get()`

If needed later:
```rust
pub fn derive<A, B, R, F>(store_a: &Store<A>, store_b: &Store<B>, f: F) -> Store<R>
where
    A: Clone + PartialEq + 'static,
    B: Clone + PartialEq + 'static,
    R: Clone + PartialEq + 'static,
    F: Fn(&A, &B) -> R + 'static,
{
    let derived = Store::new(f(&store_a.get(), &store_b.get()));
    let d1 = derived.clone();
    let a = store_a.clone();
    let b = store_b.clone();
    store_a.subscribe(move |_| d1.set(f(&a.get(), &b.get())));
    let d2 = derived.clone();
    store_b.subscribe(move |_| d2.set(f(&a.get(), &b.get())));
    derived
}
```

---

### 4. Field-Level Diff (Performance)

**Effort:** High
**Priority:** Low

When `AudioData` changes, ALL audio subscribers fire even if only `volume` changed.
Not a problem currently — PartialEq is cheap, Data structs are small.

If needed, consider:
- `#[derive(Diff)]` macro that generates per-field change detection
- Subscriber registration with field selector: `store.subscribe_field(|d| &d.volume, |vol| { ... })`

---

### 5. Batched Updates (Performance)

**Effort:** Medium
**Priority:** Low

If multiple services update simultaneously (e.g., at startup), each triggers its own notification cycle. Could batch with `glib::idle_add_local`.

Not a problem currently — no observed performance issues.

---

## Decision Record

| Date | Decision | Reason |
|------|----------|--------|
| 2026-03-30 | Keep ReactiveBool as-is | Not blocking, low priority DRY fix |
| 2026-03-30 | No unsubscribe yet | All subscriptions are lifetime-bound, no transient need |
| 2026-03-30 | No derived stores | Architecture avoids multi-store by design (good service data structs) |
| 2026-03-30 | No field-level diff | Not needed for current Data struct sizes |
| 2026-03-30 | Reactivity system sufficient for SettingsSync | PartialEq guards in both directions prevent loops |

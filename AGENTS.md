# AGENTS.md — Axis Shell Codebase Standards

## Project Overview

Axis is a GTK4 Layer Shell panel for the niri Wayland compositor on NixOS.
Architecture: reactive store + async services + modular popup system.

---

## Core Principles

**DRY** — No duplicated logic. Extract helpers, use shared traits/utilities.
**SOLID** — Single responsibility per struct/module, open for extension via traits, proper abstraction layers.
**Clean Code** — Descriptive names, small functions, no magic numbers, meaningful abstractions.
**Modular** — Adding a new feature means creating new files, not modifying existing ones (Open/Closed Principle).

---

## Architecture

### Services (Business Logic)

Every service implements the `Service` trait:

```rust
pub trait Service: 'static {
    type Data: Clone + PartialEq + Send + 'static;
    type Cmd: Send + 'static;
    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>);
}
```

- `Data` = the reactive state (published via `data_tx`)
- `Cmd` = commands the UI sends to the service (via `cmd_tx`)
- Read-only services use `type Cmd = ()`
- Services run in background threads, never block the GTK main loop

**Registration:** Add a service in 3 files:
1. `services/mod.rs` — `pub mod foo;`
2. `app_context.rs` — `pub foo: ServiceHandle<FooData, FooCmd>`
3. `main.rs::setup_services()` — `foo: spawn_service::<FooService>()`

### Stores (Reactive State)

```rust
// Standard service handle (read + write)
pub struct ServiceHandle<D, C> { pub store: ServiceStore<D>, pub tx: Sender<C> }

// Read-only handle (no commands)
pub struct ReadOnlyHandle<D> { pub store: ServiceStore<D> }
```

- `store.get()` — read current data
- `store.subscribe(|data| { ... })` — react to changes
- Store clones are cheap (Rc-based), same underlying data
- Listeners are safe during iteration (uses `std::mem::take`)

### Popups

Implement `PopupExt` trait:

```rust
pub trait PopupExt {
    fn id(&self) -> &str;          // unique ID (e.g. "qs", "launcher")
    fn base(&self) -> &PopupBase;

    fn on_open(&self) {}           // called before popup is shown
    fn on_close(&self) {}          // called before popup is hidden
    fn handle_escape(&self) { self.close(); }  // customize escape behavior

    // Default implementations provided:
    // is_open(), open(), close(), toggle()
}
```

**Registration:** In `main.rs`:
```rust
let my_popup = Rc::new(MyPopup::new(app, ctx.clone()));
shell_ctrl.register(&my_popup);  // wires Escape + visibility notify automatically
```

**PopupBase constructors:**
- `PopupBase::new(app, title, anchor_right)` — left or right anchored
- `PopupBase::new_centered(app, title)` — centered (no left/right anchor)

**Do NOT:**
- Wire Escape handlers manually (register() does it)
- Call `connect_visible_notify` manually (register() does it)
- Pass `on_state_change` callbacks to constructors (register() handles it)

### Widgets / Tiles

Use `ToggleTile::wire_service()` for toggle tiles backed by a service:

```rust
ToggleTile::wire_service(&tile, &ctx.service,
    |on| ServiceCmd::Toggle(on),     // command constructor
    |d| d.is_enabled,                // active state extractor
    open_subpage,                    // arrow click handler
    |tile, data| { ... },            // extra subscribe logic
);
```

### Bar

Bar sections are private fields with accessor methods. Click handlers are wired externally:
```rust
bar.launcher_island()  // &gtk4::Box
bar.status_island()    // &gtk4::Box
bar.workspace_island() // &gtk4::Box
bar.clock_island()     // &gtk4::Box
bar.volume_icon()      // &gtk4::Image
```

---

## Code Style

### Imports
- Group: std → external crates → crate modules
- Use `use` for frequently accessed types, full paths for one-off uses

### Naming
- Services: `{Name}Service`, `{Name}Data`, `{Name}Cmd`
- Widgets: `{Name}Popup`, `{Name}Page`, `{Name}Tile`
- Callback clones: `foo_c`, `foo_clone` suffixes

### Error Handling
- Service errors: `log::warn!` + continue (don't crash the shell)
- Background threads: `let _ = tx.send(...)` for fire-and-forget
- Never panic on missing optional resources (compositor, D-Bus, files)

### GTK Patterns
- Clone GTK widgets freely (refcounted, cheap)
- Use `Rc<>` for non-GTK structs in closures
- `connect_clicked`, `subscribe` closures capture cloned handles
- Use `glib::timeout_add_local` for async GTK work (not `std::thread`)

### CSS
- Per-widget CSS in `style.css` (will be split into modules later)
- CSS class naming: `.component-element` (e.g. `.tile-main`, `.qs-entry`)
- No inline styles in Rust code

---

## File Organization

```
src/
├── main.rs              # Entry point, setup_services(), UI wiring
├── app_context.rs       # AppContext struct, spawn_service helpers
├── store.rs             # Store, ServiceHandle, ReadOnlyHandle
├── shell/mod.rs         # PopupExt, ShellController
├── style.css            # All CSS
├── constants.rs         # Shared constants
├── services/
│   ├── mod.rs           # Service trait, pub mod declarations
│   ├── {name}.rs        # One file per service
│   └── tasks/           # Subsystem with multiple files
│       ├── mod.rs       # TaskRegistry
│       ├── provider.rs  # TaskProvider trait
│       ├── google.rs    # GoogleTasksProvider
│       ├── local.rs     # LocalTodoProvider
│       └── utils.rs     # Shared HTTP/JSON helpers
├── widgets/
│   ├── mod.rs           # pub use re-exports
│   ├── base/mod.rs      # PopupBase
│   ├── bar/             # Bar + sections
│   ├── components/      # Reusable components (ToggleTile, Island, etc.)
│   ├── {popup}/         # Each popup as a module directory or file
│   └── notification/    # Toast + Archive
```

---

## Adding New Features

### New Service
1. Create `services/foo.rs` with `FooData`, `FooCmd`, `FooService`
2. `pub mod foo;` in `services/mod.rs`
3. Add `pub foo: ServiceHandle<FooData, FooCmd>` to `AppContext`
4. `foo: spawn_service::<FooService>()` in `setup_services()`

### New Popup
1. Create `widgets/foo_popup.rs` (or `widgets/foo_popup/mod.rs`)
2. `impl PopupExt for FooPopup` with `id()` and `base()`
3. In `main.rs`: `let foo = Rc::new(FooPopup::new(app, ctx.clone()));`
4. `shell_ctrl.register(&foo);`

### New Toggle Tile
1. Create the `ToggleTile` in the page
2. Call `ToggleTile::wire_service(...)` with service handle + callbacks

### New Bar Island
1. Create `widgets/bar/my_island.rs`
2. Return a container widget
3. Wire click handler via `setup_click_handler(bar.some_island(), ctrl, "id")`

---

## What NOT to Do

- **No global statics** for UI state (use ServiceHandle + Store)
- **No blocking I/O** on the GTK main thread (use `std::thread::spawn` or async)
- **No `pub` struct fields** for widget internals (use accessor methods)
- **No duplicated match blocks** (extract shared logic into functions)
- **No manual Escape/visibility wiring** in popups (use `shell_ctrl.register()`)
- **No `is_local()` type flags** on traits (use `is_async()` or trait-based dispatch)
